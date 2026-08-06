use crate::*;

use super::super::machine::{CurrentBlock, ParseError, Parser};
use super::super::validate::*;

impl Parser<'_> {
    pub(in crate::parser) fn from(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        if self.current.is_some() {
            return Err(ParseError::new(
                line,
                source,
                "FROM is a prelude declaration and must appear before the first block",
            ));
        }
        let fields = arguments.split_whitespace().collect::<Vec<_>>();
        let Some(as_index) = fields.iter().position(|field| *field == "AS") else {
            if fields.len() == 2 {
                return Err(ParseError::new(
                    line,
                    source,
                    format!(
                        "FROM is missing AS before binder {:?}; write FROM {} AS {}",
                        fields[1], fields[0], fields[1]
                    ),
                ));
            }
            return Err(ParseError::new(
                line,
                source,
                "FROM requires an explicit binder: FROM <flakeref|index-ref:tag> AS <name>",
            ));
        };
        if as_index + 2 != fields.len() {
            return Err(ParseError::new(
                line,
                source,
                "FROM requires one binder after AS: FROM <flakeref> [OVERLAY <./file.nix>…] AS <name>",
            ));
        }
        let (url, kind) = normalize_input(fields[0], line, source)?;
        let overlay_fields = &fields[1..as_index];
        let mut overlays = Vec::new();
        for pair in overlay_fields.chunks(2) {
            if pair.len() != 2 || pair[0] != "OVERLAY" {
                return Err(ParseError::new(
                    line,
                    source,
                    "FROM overlays use repeatable OVERLAY <./file.nix> before AS",
                ));
            }
            let path = pair[1];
            if !path.starts_with("./") || path.len() <= 2 || path.contains("${") {
                return Err(ParseError::new(
                    line,
                    source,
                    "FROM OVERLAY must name a project-local ./file.nix path; overlays cannot reference Cixfile binders",
                ));
            }
            overlays.push(path.to_owned());
        }
        if !overlays.is_empty() && kind != InputKind::PackageUniverse {
            return Err(ParseError::new(
                line,
                source,
                "FROM OVERLAY applies only to a package universe; wrap the base or use a full universe tree",
            ));
        }
        let name = fields[as_index + 1];
        validate_namespace(name, line, source)?;
        self.declare_name(name, "FROM binder", line, source)?;
        self.inputs.insert(
            name.to_owned(),
            Input {
                url,
                kind,
                overlays,
                line,
            },
        );
        Ok(())
    }

    pub(in crate::parser) fn fetch(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        if let Some(CurrentBlock::Builder(name)) = self.current.clone() {
            return self.builder_command(&name, true, line, source, arguments);
        }
        if let Some(CurrentBlock::Artifact(name)) = &self.current {
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "FETCH is not legal inside {} blocks; use a top-level named FETCH or a BUILDER",
                    self.artifacts[name].kind.keyword()
                ),
            ));
        }
        let (name, remainder) = arguments.split_once(char::is_whitespace).ok_or_else(|| {
          ParseError::new(
              line,
              source,
              "top-level FETCH requires a binder and command: FETCH <name> <command…> [EXPECT <sri-hash>]",
          )
      })?;
        validate_namespace(name, line, source)?;
        let (expected, command) = parse_fetch_expect(remainder.trim(), line, source)?;
        if command.is_empty() {
            return Err(ParseError::new(
                line,
                source,
                "top-level FETCH requires a command after its binder",
            ));
        }
        let command = self.build_template(command, line, source, false)?;
        self.declare_name(name, "FETCH binder", line, source)?;
        self.fetches.insert(
            name.to_owned(),
            Fetch {
                expected,
                command,
                line,
                source: source.to_owned(),
            },
        );
        self.fetch_order.push(name.to_owned());
        Ok(())
    }
}
