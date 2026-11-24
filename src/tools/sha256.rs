//! Raw SHA-256 hex digest, shared by the Slice D receipt tools.
//!
//! This is a thin wrapper over the already-vendored `sha2` crate. It is
//! deliberately *not* [`crate::Digest`]: that type is domain-separated and is
//! used for evidence identities, whereas the receipt tools need the plain digest
//! of a file's bytes (the same thing `sha256sum` and Python's `hashlib` print).
//!
//! **Not on the verdict path.**

use sha2::{Digest as _, Sha256};

/// Lowercase hex SHA-256 of `bytes` (64 characters).
pub fn hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(char::from_digit((byte >> 4) as u32, 16).unwrap_or('0'));
        out.push(char::from_digit((byte & 0xf) as u32, 16).unwrap_or('0'));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::hex;

    #[test]
    fn matches_known_vectors() {
        assert_eq!(
            hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
