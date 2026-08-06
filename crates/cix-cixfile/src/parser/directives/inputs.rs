use crate::*;

use super::super::machine::{CurrentBlock, ParseError, Parser};
use super::super::validate::*;

impl Parser<'_> {
    pub(in crate::parser) fn let_binding(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        if self.current.is_some() {
            return Err(ParseError::new(
                line,
                source,
                "LET is a prelude declaration and must appear before the first block",
            ));
        }
        let (name, values) = arguments
            .split_once('=')
            .ok_or_else(|| ParseError::new(line, source, "LET syntax is LET NAME = value"))?;
        let name = name.trim();
        validate_namespace(name, line, source)?;
        let values = argv_fields(values.trim(), line, source, "LET")?;
        if values.len() != 1 {
            return Err(ParseError::new(
                line,
                source,
                "multi-value LET is reserved; declare one value in this epoch",
            ));
        }
        if self.lets.insert(name.to_owned(), values).is_some()
            || self.args.contains_key(name)
            || self.names.contains_key(name)
        {
            return Err(ParseError::new(
                line,
                source,
                format!("name {name:?} is already declared"),
            ));
        }
        Ok(())
    }

    pub(in crate::parser) fn arg_binding(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        if self.current.is_some() {
            return Err(ParseError::new(
                line,
                source,
                "ARG is a prelude declaration and must appear before the first block",
            ));
        }
        let (name, values) = arguments.split_once(" from ").ok_or_else(|| {
            ParseError::new(line, source, "ARG syntax is ARG NAME from value1 value2 …")
        })?;
        let name = name.trim();
        validate_namespace(name, line, source)?;
        let values = argv_fields(values.trim(), line, source, "ARG")?;
        let selected = values
            .first()
            .expect("argv_fields requires one value")
            .clone();
        if self
            .args
            .insert(
                name.to_owned(),
                Arg {
                    values,
                    selected,
                    line,
                },
            )
            .is_some()
            || self.lets.contains_key(name)
            || self.names.contains_key(name)
        {
            return Err(ParseError::new(
                line,
                source,
                format!("name {name:?} is already declared"),
            ));
        }
        Ok(())
    }

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
        let command = if let Some((interpreter, delimiter)) = run_heredoc(command, line, source)? {
            NodeCommand::Heredoc {
                interpreter: self.build_template(&interpreter, line, source, false)?,
                body: self.read_heredoc_body("FETCH", &delimiter, line, source)?,
            }
        } else if self.opened_block.is_none() && legacy_shell_form(command) {
            NodeCommand::Legacy(self.build_template(command, line, source, false)?)
        } else {
            let argv = argv_fields(command, line, source, "FETCH")?;
            reject_shell_variable(&argv, line, source)?;
            NodeCommand::Argv(
                argv.into_iter()
                    .map(|arg| self.build_template(&arg, line, source, false))
                    .collect::<Result<_, _>>()?,
            )
        };
        let (environment, ignored_evidence, clause_expected) = self.node_clauses(line, source)?;
        let expected = clause_expected.or(expected);
        self.declare_name(name, "FETCH binder", line, source)?;
        self.fetches.insert(
            name.to_owned(),
            Fetch {
                expected,
                command,
                environment,
                ignored_evidence,
                line,
                source: source.to_owned(),
            },
        );
        self.fetch_order.push(name.to_owned());
        Ok(())
    }
}

fn legacy_shell_form(arguments: &str) -> bool {
    arguments.contains(['|', '&', ';', '>', '<', '`']) || arguments.contains('$')
}
