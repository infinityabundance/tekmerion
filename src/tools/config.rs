//! The effective execution configuration (XVI.C).
//!
//! A `TEKMERION_*` variable can change what a run proves without changing any
//! file hash or executable digest, so a freeze that binds only hashes does not
//! bind the reasoning path. This module declares every such variable in one
//! place, captures its value at run time, records it in a run record and in the
//! freeze closure, and compares a recorded configuration against the live one.
//!
//! The variables are grouped by *who reads them*, because a single grep cannot
//! see all three programs:
//!
//! * [`ENV_VARS`] — read by the Rust binary (`std::env::var`/`var_os` in
//!   `src/`).
//! * [`BRIDGE_ENV_VARS`] — read by the OCaml bridge under `tools/`.
//! * [`TEST_ENV_VARS`] — read by the fixture recorder under `tests/`.
//!
//! [`ALL_ENV_VARS`] is the union that the freeze binds. A test greps each
//! source for its group and fails if the declaration and the source disagree,
//! so a new variable cannot appear unnoticed.
//!
//! **Not on the verdict path.**

use crate::tools::guard::{Refused, Result};
use std::collections::BTreeMap;

/// The recorded value of a variable that is not set.
///
/// A variable whose literal value is the string `unset` is indistinguishable
/// from an unset one in the recorded form. Every declared `TEKMERION_*`
/// variable carries a path, a task id or a flag token, so this is not observed;
/// it is stated here rather than hidden.
pub const UNSET: &str = "unset";

/// Variables the **Rust binary** reads. Asserted against `src/` by
/// `declared_binary_vars_match_the_source`.
pub const ENV_VARS: &[&str] = &[
    "TEKMERION_ALG_CLOSE",
    "TEKMERION_FAULT",
    "TEKMERION_PROBLEMS",
    "TEKMERION_SIM_BACKEND",
];

/// Variables the **OCaml bridge** (`tools/*.ml`, invoked by the emitter) reads.
/// Asserted against `tools/bridge.ml` by `declared_bridge_vars_match_the_bridge`.
pub const BRIDGE_ENV_VARS: &[&str] = &["TEKMERION_SLICE_CONTROL", "TEKMERION_SLICE_DEBUG"];

/// Variables the **fixture recorder test** reads. Asserted against `tests/` by
/// `declared_test_vars_match_the_recorder`.
pub const TEST_ENV_VARS: &[&str] = &["TEKMERION_RECORD_ANSWERS"];

/// Every declared variable, sorted. This is the set the freeze binds and
/// verifies; nothing outside this list is read by Tekmerion's own code.
pub const ALL_ENV_VARS: &[&str] = &[
    "TEKMERION_ALG_CLOSE",
    "TEKMERION_FAULT",
    "TEKMERION_PROBLEMS",
    "TEKMERION_RECORD_ANSWERS",
    "TEKMERION_SIM_BACKEND",
    "TEKMERION_SLICE_CONTROL",
    "TEKMERION_SLICE_DEBUG",
];

/// The behavioural flags that change the reasoning path but are not
/// environment variables. `None` means the caller did not state the flag; it is
/// recorded as `unspecified`, never guessed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFlags {
    /// The emitter's `--mode` (`answer`, `dev`, `phase`, `cert-producer`).
    pub mode: Option<String>,
    /// The proof was wrapped with the axiom-stamp instrument (`--stamp`).
    pub stamp: Option<bool>,
    /// The build was forced (`--force` / `make -B`).
    pub force: Option<bool>,
    /// A slice control is in force (`TEKMERION_SLICE_CONTROL` set non-empty).
    pub slice: bool,
}

impl RunFlags {
    /// The flags a drop-in producer knows: it always knows stamp/force, never
    /// knows the emitter's `--mode`.
    pub fn producer(stamp: bool, force: bool) -> RunFlags {
        RunFlags {
            mode: None,
            stamp: Some(stamp),
            force: Some(force),
            slice: slice_on(),
        }
    }
}

/// Whether a slice control is in force.
pub fn slice_on() -> bool {
    live_value(SLICE_CONTROL).is_some_and(|v| !v.is_empty())
}

const SLICE_CONTROL: &str = "TEKMERION_SLICE_CONTROL";

/// A captured effective configuration: every declared variable (with `None`
/// for unset) plus the behavioural flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveConfig {
    env: BTreeMap<String, Option<String>>,
    flags: RunFlags,
}

impl EffectiveConfig {
    /// Construct from already-parsed parts, refusing an incomplete or
    /// over-complete variable set. This is the only way to build one from a
    /// manifest, so a manifest cannot drop a variable silently.
    pub fn from_entries(
        env: BTreeMap<String, Option<String>>,
        flags: RunFlags,
    ) -> Result<EffectiveConfig> {
        for name in ALL_ENV_VARS {
            if !env.contains_key(*name) {
                return Err(Refused::new(format!(
                    "effective configuration is missing {name}"
                )));
            }
        }
        for name in env.keys() {
            if !ALL_ENV_VARS.contains(&name.as_str()) {
                return Err(Refused::new(format!(
                    "effective configuration names undeclared variable {name}"
                )));
            }
        }
        Ok(EffectiveConfig { env, flags })
    }

    /// Read the live environment. `flags` carries what the caller knows.
    pub fn capture(flags: RunFlags) -> EffectiveConfig {
        let env = ALL_ENV_VARS
            .iter()
            .map(|name| ((*name).to_string(), live_value(name)))
            .collect();
        EffectiveConfig { env, flags }
    }

    pub fn env(&self) -> &BTreeMap<String, Option<String>> {
        &self.env
    }

    pub fn flags(&self) -> &RunFlags {
        &self.flags
    }

    /// The recorded value, `None` for unset.
    pub fn value(&self, name: &str) -> Option<&Option<String>> {
        self.env.get(name)
    }
}

/// The live value of one variable, lossily decoded. An empty string is a set
/// value and is distinct from unset.
pub fn live_value(name: &str) -> Option<String> {
    std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
}

fn shown(value: Option<&str>) -> String {
    match value {
        Some(text) => format!("{text:?}"),
        None => UNSET.to_string(),
    }
}

/// Compare a recorded configuration with live values supplied by `live`.
/// Refuses on the first mismatch and names the variable. Pure, so the firing
/// controls do not have to mutate the test process's environment.
pub fn verify_against<F>(recorded: &EffectiveConfig, live: F) -> Result<()>
where
    F: Fn(&str) -> Option<String>,
{
    for name in ALL_ENV_VARS {
        let want = recorded.env.get(*name).cloned().flatten();
        let got = live(name);
        if want != got {
            return Err(Refused::new(format!(
                "configuration mismatch: {name} recorded {}, live {}",
                shown(want.as_deref()),
                shown(got.as_deref())
            )));
        }
    }
    Ok(())
}

/// Compare a recorded configuration with the live process environment.
pub fn verify_live(recorded: &EffectiveConfig) -> Result<()> {
    verify_against(recorded, live_value)
}

/// The canonical JSON fragment. Keys are emitted in [`ALL_ENV_VARS`] order
/// (which is sorted), so the rendering is deterministic.
pub fn render_json(config: &EffectiveConfig) -> String {
    let mut env = String::from("{");
    for (i, name) in ALL_ENV_VARS.iter().enumerate() {
        if i > 0 {
            env.push_str(", ");
        }
        env.push_str(&quote(name));
        env.push_str(": ");
        match config.env.get(*name).cloned().flatten() {
            Some(value) => env.push_str(&quote(&value)),
            None => env.push_str(&quote(UNSET)),
        }
    }
    env.push('}');
    let flags = &config.flags;
    format!(
        "{{\"env\": {env}, \"flags\": {{\"mode\": {}, \"stamp\": {}, \"force\": {}, \"slice\": {}}}}}",
        quote(flags.mode.as_deref().unwrap_or("unspecified")),
        tri(flags.stamp),
        tri(flags.force),
        if flags.slice { "\"on\"" } else { "\"off\"" },
    )
}

fn tri(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "\"on\"",
        Some(false) => "\"off\"",
        None => "\"unspecified\"",
    }
}

fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Parse the flag token used in a manifest: `on`, `off` or `unspecified`.
pub fn parse_tri(text: &str, where_: &str) -> Result<Option<bool>> {
    match text {
        "on" => Ok(Some(true)),
        "off" => Ok(Some(false)),
        "unspecified" => Ok(None),
        other => Err(Refused::new(format!(
            "{where_} must be on, off or unspecified: {other:?}"
        ))),
    }
}

/// A short human summary, one variable per line, for a receipt or a log.
pub fn describe(config: &EffectiveConfig) -> String {
    let mut out = String::new();
    for name in ALL_ENV_VARS {
        let value = config.env.get(*name).cloned().flatten();
        out.push_str(&format!("{name}={}\n", shown(value.as_deref())));
    }
    let flags = &config.flags;
    out.push_str(&format!(
        "mode={} stamp={} force={} slice={}\n",
        flags.mode.as_deref().unwrap_or("unspecified"),
        flag_text(flags.stamp),
        flag_text(flags.force),
        if flags.slice { "on" } else { "off" },
    ));
    out
}

fn flag_text(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "on",
        Some(false) => "off",
        None => "unspecified",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    // These needles are built at compile time so this test's own source does
    // not contain the patterns it searches for.
    const CALL_VAR: &str = concat!("env::", "var(");
    const CALL_VAR_OS: &str = concat!("env::", "var_os(");
    const SYS_GETENV: &str = concat!("Sys.", "getenv \"");

    fn walk(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, ext, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some(ext) {
                out.push(path);
            }
        }
    }

    fn read_all(paths: &[PathBuf]) -> Vec<(PathBuf, String)> {
        paths
            .iter()
            .map(|p| (p.clone(), fs::read_to_string(p).expect("read source")))
            .collect()
    }

    /// The argument text of `env::var(..)` / `env::var_os(..)` on one line.
    fn env_arg(line: &str) -> Option<String> {
        let idx = line.find(CALL_VAR_OS).or_else(|| line.find(CALL_VAR))?;
        let after = &line[idx..];
        let open = after.find('(')?;
        let rest = &after[open + 1..];
        let end = rest.find(')')?;
        Some(
            rest[..end]
                .trim()
                .trim_start_matches('&')
                .trim()
                .to_string(),
        )
    }

    fn tekm_literal(arg: &str) -> Option<String> {
        let body = arg.strip_prefix('"')?.strip_suffix('"')?;
        if body.starts_with("TEKMERION_")
            && body
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        {
            Some(body.to_string())
        } else {
            None
        }
    }

    /// `const IDENT: &str = "TEKMERION_...";`, including through `pub`.
    fn const_env(line: &str) -> Option<(String, String)> {
        let text = line.trim_start();
        let text = text.strip_prefix("pub ").unwrap_or(text);
        let text = text.strip_prefix("const ")?;
        let (ident, rest) = text.split_once(':')?;
        let open = rest.find('"')?;
        let after = &rest[open + 1..];
        let close = after.find('"')?;
        let value = &after[..close];
        if value.starts_with("TEKMERION_") {
            Some((ident.trim().to_string(), value.to_string()))
        } else {
            None
        }
    }

    fn declared(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn declared_binary_vars_match_the_source() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        walk(&root.join("src"), "rs", &mut files);
        let sources = read_all(&files);

        let mut consts: BTreeMap<String, String> = BTreeMap::new();
        for (_, text) in &sources {
            for line in text.lines() {
                if let Some((ident, value)) = const_env(line) {
                    consts.insert(ident, value);
                }
            }
        }

        let mut found = BTreeSet::new();
        for (path, text) in &sources {
            for line in text.lines() {
                let Some(arg) = env_arg(line) else {
                    continue;
                };
                if arg.starts_with('"') {
                    // A literal: only a TEKMERION_* one is declared.
                    if let Some(value) = tekm_literal(&arg) {
                        found.insert(value);
                    }
                    continue;
                }
                // An identifier. Upper-case identifiers are constants that must
                // resolve; lower-case ones are function parameters (`name` in
                // config.rs's own `live_value`).
                if !arg.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                    continue;
                }
                match consts.get(&arg) {
                    Some(value) => {
                        found.insert(value.clone());
                    }
                    None => panic!(
                        "{}: cannot resolve the constant {arg:?} of an env read; \
                         declare it in src/tools/config.rs",
                        path.display()
                    ),
                }
            }
        }

        assert_eq!(
            found,
            declared(ENV_VARS),
            "the TEKMERION_* variables the binary reads and ENV_VARS disagree"
        );
    }

    #[test]
    fn declared_bridge_vars_match_the_bridge() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = root.join("tools").join("bridge.ml");
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let mut found = BTreeSet::new();
        for line in text.lines() {
            let Some(idx) = line.find(SYS_GETENV) else {
                continue;
            };
            let after = &line[idx + SYS_GETENV.len()..];
            let Some(close) = after.find('"') else {
                continue;
            };
            let name = &after[..close];
            found.insert(name.to_string());
        }
        assert_eq!(
            found,
            declared(BRIDGE_ENV_VARS),
            "the TEKMERION_* variables tools/bridge.ml reads and BRIDGE_ENV_VARS disagree"
        );
    }

    #[test]
    fn declared_test_vars_match_the_recorder() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        walk(&root.join("tests"), "rs", &mut files);
        let mut found = BTreeSet::new();
        for (_, text) in read_all(&files) {
            for line in text.lines() {
                let Some(arg) = env_arg(line) else {
                    continue;
                };
                if let Some(value) = tekm_literal(&arg) {
                    found.insert(value);
                }
            }
        }
        assert_eq!(
            found,
            declared(TEST_ENV_VARS),
            "the TEKMERION_* variables the tests read and TEST_ENV_VARS disagree"
        );
    }

    fn sample() -> EffectiveConfig {
        let mint = |stm: bool, stm_exec: bool| RunFlags {
            mode: Some("answer".to_string()),
            stamp: Some(stm),
            force: Some(stm_exec),
            slice: false,
        };
        let mut env = BTreeMap::new();
        env.insert("TEKMERION_ALG_CLOSE".to_string(), None);
        env.insert("TEKMERION_FAULT".to_string(), Some("L01".to_string()));
        env.insert("TEKMERION_PROBLEMS".to_string(), None);
        env.insert("TEKMERION_RECORD_ANSWERS".to_string(), None);
        env.insert("TEKMERION_SIM_BACKEND".to_string(), None);
        env.insert(
            "TEKMERION_SLICE_CONTROL".to_string(),
            Some("drop-largest".to_string()),
        );
        env.insert("TEKMERION_SLICE_DEBUG".to_string(), None);
        EffectiveConfig::from_entries(env, mint(true, false)).expect("sample config")
    }

    #[test]
    fn verify_against_accepts_a_match_and_names_a_mismatch() {
        let recorded = sample();
        let live = |name: &str| -> Option<String> {
            match name {
                "TEKMERION_FAULT" => Some("L01".to_string()),
                "TEKMERION_SLICE_CONTROL" => Some("drop-largest".to_string()),
                _ => None,
            }
        };
        verify_against(&recorded, live).expect("matching configuration verifies");

        // A variable that was recorded unset and is now set.
        let flipped = |name: &str| -> Option<String> {
            match name {
                "TEKMERION_ALG_CLOSE" => Some("arm.bignum_add_p25519".to_string()),
                "TEKMERION_FAULT" => Some("L01".to_string()),
                "TEKMERION_SLICE_CONTROL" => Some("drop-largest".to_string()),
                _ => None,
            }
        };
        let err = verify_against(&recorded, flipped).unwrap_err().to_string();
        assert!(err.contains("TEKMERION_ALG_CLOSE"), "{err}");

        // A variable that was recorded set and is now unset.
        let unset = |name: &str| -> Option<String> {
            match name {
                "TEKMERION_FAULT" => None,
                "TEKMERION_SLICE_CONTROL" => Some("drop-largest".to_string()),
                _ => None,
            }
        };
        let err = verify_against(&recorded, unset).unwrap_err().to_string();
        assert!(err.contains("TEKMERION_FAULT"), "{err}");
        assert!(err.contains("recorded \"L01\""), "{err}");
        assert!(err.contains("live unset"), "{err}");
    }

    #[test]
    fn an_incomplete_or_over_complete_set_is_refused() {
        let mut env = BTreeMap::new();
        env.insert("TEKMERION_ALG_CLOSE".to_string(), None);
        let err = EffectiveConfig::from_entries(env, RunFlags::producer(false, false))
            .unwrap_err()
            .to_string();
        assert!(err.contains("missing TEKMERION_FAULT"), "{err}");

        let mut over = BTreeMap::new();
        for name in ALL_ENV_VARS {
            over.insert((*name).to_string(), None);
        }
        over.insert("TEKMERION_NOT_DECLARED".to_string(), None);
        let err = EffectiveConfig::from_entries(over, RunFlags::producer(false, false))
            .unwrap_err()
            .to_string();
        assert!(err.contains("undeclared variable"), "{err}");
    }

    #[test]
    fn render_json_round_trips_the_declared_set() {
        let text = render_json(&sample());
        assert!(
            text.contains("\"TEKMERION_ALG_CLOSE\": \"unset\""),
            "{text}"
        );
        assert!(
            text.contains("\"TEKMERION_SLICE_CONTROL\": \"drop-largest\""),
            "{text}"
        );
        assert!(text.contains("\"slice\": \"off\""), "{text}");
        let parsed = crate::tools::json::parse(text.as_bytes()).expect("valid JSON");
        let env = parsed.get("env").unwrap().as_obj().unwrap();
        assert_eq!(env.len(), ALL_ENV_VARS.len());
        assert_eq!(env.get("TEKMERION_PROBLEMS").unwrap().as_str(), Some(UNSET));
    }

    #[test]
    fn describe_prints_unset_and_the_flags() {
        let text = describe(&sample());
        assert!(text.contains("TEKMERION_ALG_CLOSE=unset"), "{text}");
        assert!(text.contains("TEKMERION_FAULT=\"L01\""), "{text}");
        assert!(
            text.contains("mode=answer stamp=on force=off slice=off"),
            "{text}"
        );
    }
}
