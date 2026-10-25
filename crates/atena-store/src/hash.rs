//! Fingerprints: BLAKE3 hashes of what a publisher served and of normalised texts.
//!
//! ATENA keeps original files only in its archive of official and openly licensed documents
//! (pillars 2 and 3: a file may carry harmful content or personal data that cannot be removed from
//! it reliably). The hash of the original bytes is kept with every version: it proves what was
//! served, and anyone holding a copy can verify it.

/// A 32-byte BLAKE3 digest.
pub type Hash = [u8; 32];

/// The hash of `bytes`.
pub fn hash(bytes: &[u8]) -> Hash {
    *blake3::hash(bytes).as_bytes()
}

/// Lower-case hexadecimal form of a hash.
pub fn hex(hash: &Hash) -> String {
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

/// A hash from its hexadecimal form.
pub fn parse_hex(text: &str) -> Option<Hash> {
    if text.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_round_trip_through_hex() {
        let h = hash(b"atena");
        assert_eq!(parse_hex(&hex(&h)), Some(h));
        assert_eq!(parse_hex("zz"), None);
        assert_eq!(parse_hex(&"g".repeat(64)), None);
    }
}
