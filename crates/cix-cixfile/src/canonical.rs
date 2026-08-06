//! Stable semantic serialization for Cixfile-derived keys.

use crate::Cixfile;

/// Bump this when the canonical AST representation changes.
pub const KEY_SERIALIZATION_VERSION: &str = "cixfile-canonical-ast-v1";

/// Serializes parser semantics while excluding diagnostic source locations.
///
/// The parser model uses ordered vectors where declaration order is meaningful
/// and ordered maps where names are not. `serde_json` preserves both choices,
/// making formatting, comments, and source locations key-neutral.
pub fn serialize(cixfile: &Cixfile) -> serde_json::Result<Vec<u8>> {
    let mut bytes = KEY_SERIALIZATION_VERSION.as_bytes().to_vec();
    bytes.push(0);
    bytes.extend(serde_json::to_vec(cixfile)?);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::serialize;
    use crate::parse;

    #[test]
    fn serialization_is_formatter_and_comment_neutral_but_semantic() {
        let messy = "# an ignored comment\nFROM\tgithub:NixOS/nixpkgs/nixos-unstable\tAS\tpkgs\nSERVICE\tapp\nSTART\t/bin/true\n";
        let formatted = crate::fmt::format(messy).unwrap();
        assert_eq!(
            serialize(&parse(messy).unwrap()).unwrap(),
            serialize(&parse(&formatted).unwrap()).unwrap()
        );

        let changed = formatted.replace("/bin/true", "/bin/false");
        assert_ne!(
            serialize(&parse(&formatted).unwrap()).unwrap(),
            serialize(&parse(&changed).unwrap()).unwrap()
        );
    }
}
