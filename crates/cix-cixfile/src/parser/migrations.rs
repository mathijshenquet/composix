//! Migration-only directive rewrites kept separate from the live grammar.

use super::machine::ParseError;
use super::validate::valid_attrpath;

pub(super) fn take_removed_error(line: usize, source: &str, arguments: &str) -> ParseError {
    let mut fields = arguments.split_whitespace();
    let rewrite = match (fields.next(), fields.next(), fields.next()) {
       (Some(from), Some(to), None) => format!(
           "TAKE was removed; inside the artifact block write COPY ${{build}}/{from} {to}, with a producing `BUILDER build`; see docs/cixfile.md#copy"
       ),
       _ => "TAKE was removed; use COPY ${<builder>}/<path> <destination> inside the artifact block; see docs/cixfile.md#copy".to_owned(),
   };
    ParseError::new(line, source, rewrite)
}

pub(super) fn pkg_removed_error(line: usize, source: &str, arguments: &str) -> ParseError {
    let rewrite = arguments
       .split_whitespace()
       .next()
       .filter(|attribute| valid_attrpath(attribute))
       .map_or_else(
           || "PKG was removed; reference packages directly as ${pkgs.<attrpath>}; see docs/cixfile.md#inputs"
               .to_owned(),
           |attribute| {
               format!(
                   "PKG was removed; delete this line and replace ${{{attribute}}} with ${{pkgs.{attribute}}}; see docs/cixfile.md#inputs"
               )
           },
       );
    ParseError::new(line, source, rewrite)
}
