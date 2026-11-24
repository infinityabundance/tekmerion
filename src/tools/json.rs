//! Minimal strict JSON reader for tooling. Not on the verdict path.
//!
//! The corpus (`problems.json`) is machine-generated UTF-8 JSON. This parser
//! accepts exactly RFC 8259 JSON (including `\uXXXX` and surrogate pairs) and
//! refuses trailing bytes, duplicate object keys and unescaped control bytes.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_obj(&self) -> Option<&BTreeMap<String, Json>> {
        match self {
            Json::Obj(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(v) => Some(v),
            _ => None,
        }
    }
}

pub fn parse(bytes: &[u8]) -> Result<Json, String> {
    let s = std::str::from_utf8(bytes).map_err(|_| "input is not UTF-8".to_string())?;
    let mut p = Parser {
        s: s.as_bytes(),
        i: 0,
    };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.s.len() {
        return Err(format!("trailing bytes at offset {}", p.i));
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn eat(&mut self, want: u8) -> Result<(), String> {
        if self.peek() == Some(want) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected {:?} at offset {}", want as char, self.i))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        match self.peek() {
            Some(b'n') => self.lit(b"null", Json::Null),
            Some(b't') => self.lit(b"true", Json::Bool(true)),
            Some(b'f') => self.lit(b"false", Json::Bool(false)),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(c) => Err(format!(
                "unexpected byte {:?} at offset {}",
                c as char, self.i
            )),
            None => Err("unexpected end of input".to_string()),
        }
    }

    fn lit(&mut self, text: &[u8], out: Json) -> Result<Json, String> {
        if self.s[self.i..].starts_with(text) {
            self.i += text.len();
            Ok(out)
        } else {
            Err(format!("bad literal at offset {}", self.i))
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.eat(b'[')?;
        self.ws();
        let mut out = Vec::new();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Json::Arr(out));
        }
        loop {
            self.ws();
            out.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                }
                Some(b']') => {
                    self.i += 1;
                    return Ok(Json::Arr(out));
                }
                _ => return Err(format!("expected , or ] at offset {}", self.i)),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.eat(b'{')?;
        self.ws();
        let mut out = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Json::Obj(out));
        }
        loop {
            self.ws();
            let key = self.string()?;
            self.ws();
            self.eat(b':')?;
            self.ws();
            let val = self.value()?;
            if out.insert(key.clone(), val).is_some() {
                return Err(format!("duplicate object key {key:?}"));
            }
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                }
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Json::Obj(out));
                }
                _ => return Err(format!("expected , or }} at offset {}", self.i)),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out = String::new();
        loop {
            let c = self
                .peek()
                .ok_or_else(|| "unterminated string".to_string())?;
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let e = self
                        .peek()
                        .ok_or_else(|| "unterminated escape".to_string())?;
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000C}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let ch = if (0xD800..=0xDBFF).contains(&hi) {
                                if self.s[self.i..].starts_with(b"\\u") {
                                    self.i += 2;
                                    let lo = self.hex4()?;
                                    if !(0xDC00..=0xDFFF).contains(&lo) {
                                        return Err("bad low surrogate".to_string());
                                    }
                                    let cp = 0x10000
                                        + (((hi - 0xD800) as u32) << 10)
                                        + (lo - 0xDC00) as u32;
                                    char::from_u32(cp)
                                        .ok_or_else(|| "bad code point".to_string())?
                                } else {
                                    return Err("lone high surrogate".to_string());
                                }
                            } else if (0xDC00..=0xDFFF).contains(&hi) {
                                return Err("lone low surrogate".to_string());
                            } else {
                                char::from_u32(hi as u32)
                                    .ok_or_else(|| "bad code point".to_string())?
                            };
                            out.push(ch);
                        }
                        _ => return Err(format!("bad escape \\{}", e as char)),
                    }
                }
                0x00..=0x1F => return Err("unescaped control byte in string".to_string()),
                _ => {
                    // UTF-8 continuation: copy the full scalar.
                    let start = self.i - 1;
                    let len = utf8_len(c);
                    let end = start + len;
                    if end > self.s.len() {
                        return Err("truncated UTF-8".to_string());
                    }
                    let piece = std::str::from_utf8(&self.s[start..end])
                        .map_err(|_| "invalid UTF-8 in string".to_string())?;
                    out.push_str(piece);
                    self.i = end;
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u16, String> {
        if self.i + 4 > self.s.len() {
            return Err("truncated \\u escape".to_string());
        }
        let mut v: u16 = 0;
        for _ in 0..4 {
            let c = self.s[self.i];
            self.i += 1;
            let d = match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                _ => return Err("bad hex digit in \\u escape".to_string()),
            };
            v = (v << 4) | d as u16;
        }
        Ok(v)
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        while matches!(
            self.peek(),
            Some(b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
        ) {
            self.i += 1;
        }
        let text =
            std::str::from_utf8(&self.s[start..self.i]).map_err(|_| "bad number".to_string())?;
        text.parse::<f64>()
            .map(Json::Num)
            .map_err(|_| format!("bad number {text:?}"))
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_escapes_and_nesting() {
        let j = parse(br#"{"a":"x\\y\n\"z\"","b":[1,2],"c":{"d":true}}"#).unwrap();
        assert_eq!(j.get("a").unwrap().as_str().unwrap(), "x\\y\n\"z\"");
        assert_eq!(j.get("b").unwrap().as_array().unwrap().len(), 2);
        assert_eq!(j.get("c").unwrap().get("d"), Some(&Json::Bool(true)));
    }

    #[test]
    fn refuses_duplicate_keys_and_trailing_bytes() {
        assert!(parse(br#"{"a":1,"a":2}"#).is_err());
        assert!(parse(br#"{} x"#).is_err());
    }

    #[test]
    fn decodes_surrogate_pairs() {
        let j = parse(br#""\ud83d\ude00""#).unwrap();
        assert_eq!(j.as_str().unwrap(), "\u{1F600}");
    }
}
