//! Directive handlers grouped by the language stratum they mutate.

use crate::*;

use super::machine::{CurrentBlock, DeclaredName, ParseError, Parser};
use super::validate::*;

mod assembly;
mod builder;
mod contract;
mod inputs;

impl Parser<'_> {
    pub(super) fn current_service_mut(
        &mut self,
        directive: &str,
        line: usize,
        source: &str,
    ) -> Result<&mut Service, ParseError> {
        let name = self
            .current_artifact_name(directive, line, source)?
            .to_owned();
        Ok(&mut self
            .artifacts
            .get_mut(&name)
            .expect("current artifact exists")
            .service)
    }

    pub(super) fn current_artifact_name(
        &self,
        directive: &str,
        line: usize,
        source: &str,
    ) -> Result<&str, ParseError> {
        match &self.current {
            Some(CurrentBlock::Artifact(name)) => Ok(name),
            Some(CurrentBlock::Builder(_)) => Err(ParseError::new(
                line,
                source,
                format!("{directive} is not legal inside a BUILDER block"),
            )),
            None => Err(ParseError::new(
                line,
                source,
                format!("{directive} must appear inside an artifact block"),
            )),
        }
    }

    pub(super) fn current_builder_name(
        &self,
        directive: &str,
        line: usize,
        source: &str,
    ) -> Result<&str, ParseError> {
        match &self.current {
            Some(CurrentBlock::Builder(name)) => Ok(name),
            Some(CurrentBlock::Artifact(_)) => Err(ParseError::new(
                line,
                source,
                format!("{directive} is not legal inside an artifact block"),
            )),
            None => Err(ParseError::new(
                line,
                source,
                format!("{directive} must appear inside a BUILDER block"),
            )),
        }
    }

    pub(super) fn require_artifact_kind(
        &self,
        directive: &str,
        line: usize,
        source: &str,
        allowed: &[ArtifactKind],
    ) -> Result<(), ParseError> {
        let name = self.current_artifact_name(directive, line, source)?;
        let kind = self.artifacts[name].kind;
        if !kind.is_runnable() {
            return Err(self.item_seam_parse_error(directive, line, source));
        }
        if allowed.contains(&kind) {
            return Ok(());
        }
        let destination = allowed
            .iter()
            .map(|kind| kind.keyword())
            .collect::<Vec<_>>()
            .join(" or ");
        Err(ParseError::new(
          line,
          source,
          format!(
              "{directive} is not allowed inside {}; move it to {destination}; see docs/cixfile.md#blocks-and-directives",
              kind.keyword(),
          ),
      ))
    }

    pub(super) fn item_seam_error(
        &self,
        directive: &str,
        line: usize,
        source: &str,
    ) -> Option<ParseError> {
        const RUNTIME_DIRECTIVES: &[&str] = &[
            "START",
            "START_PRE",
            "ENV",
            "PORT",
            "LISTENER",
            "READINESS",
            "LIVENESS",
            "STOPSIGNAL",
            "STATEDIR",
            "CACHEDIR",
            "LOGDIR",
            "CONFIGDIR",
            "RUNDIR",
            "DIR",
            "STATE",
            "LOGS",
            "CONFIG",
            "CLAIM",
            "JIT",
            "EGRESS",
            "OUTBOUND",
            "PATH",
            "HEALTH",
        ];
        if !RUNTIME_DIRECTIVES.contains(&directive) {
            return None;
        }
        let Some(CurrentBlock::Artifact(name)) = &self.current else {
            return None;
        };
        (self.artifacts[name].kind == ArtifactKind::Item)
            .then(|| self.item_seam_parse_error(directive, line, source))
    }

    pub(super) fn item_seam_parse_error(
        &self,
        directive: &str,
        line: usize,
        source: &str,
    ) -> ParseError {
        ParseError::new(
          line,
          source,
          format!(
              "{directive} is runtime vocabulary, but ITEM is content-only; use SERVICE or APP for a runnable contract; see docs/cixfile.md#item"
          ),
      )
    }

    pub(super) fn claim_artifact_destination(
        &mut self,
        destination: &str,
        line: usize,
        source: &str,
    ) -> Result<(), ParseError> {
        let name = self
            .current_artifact_name("artifact assembly", line, source)?
            .to_owned();
        if !self
            .destinations
            .get_mut(&name)
            .expect("current artifact destinations exist")
            .insert(destination.to_owned())
        {
            return Err(ParseError::new(
                line,
                source,
                format!("artifact destination {destination:?} is already populated"),
            ));
        }
        Ok(())
    }

    pub(super) fn build_template(
        &self,
        input: &str,
        line: usize,
        source: &str,
        heredoc: bool,
    ) -> Result<Template, ParseError> {
        let template = build_template(
            input,
            line,
            source,
            heredoc,
            &self.inputs,
            &self.names,
            &self.lets,
            &self.args,
        )?;
        if let Some(current) = &self.current {
            let current = match current {
                CurrentBlock::Builder(name) | CurrentBlock::Artifact(name) => name,
            };
            if template
                .parts
                .iter()
                .any(|part| matches!(part, TemplatePart::Binder { name, .. } if name == current))
            {
                return Err(ParseError::new(
                    line,
                    source,
                    format!(
                        "binder {current:?} cannot reference itself; references are backward-only"
                    ),
                ));
            }
        }
        Ok(template)
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn node_clauses(
        &mut self,
        node_line: usize,
        node_source: &str,
    ) -> Result<
        (
            std::collections::BTreeMap<String, Template>,
            std::collections::BTreeSet<String>,
            Option<String>,
        ),
        ParseError,
    > {
        let mut environment = std::collections::BTreeMap::new();
        let mut ignored = std::collections::BTreeSet::new();
        let mut expected = None;
        while let Some(source) = self.lines.get(self.index).copied() {
            let trimmed = source.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                break;
            }
            let (directive, arguments) = trimmed
                .split_once(char::is_whitespace)
                .map_or((trimmed, ""), |(d, a)| (d, a.trim()));
            match directive {
                "WITH" => {
                    self.index += 1;
                    if let Some(path) = arguments.strip_prefix("UNSAFE IGNORE ") {
                        if path.is_empty() || path.contains(char::is_whitespace) {
                            return Err(ParseError::new(
                                self.index,
                                source,
                                "WITH UNSAFE IGNORE requires one path",
                            ));
                        }
                        ignored.insert(path.to_owned());
                        continue;
                    }
                    let (name, value) = match arguments.split_once('=') {
                        Some((name, value)) if !name.is_empty() && !value.is_empty() => {
                            (name, self.build_template(value, self.index, source, false)?)
                        }
                        None if !arguments.is_empty() => {
                            let values = self.lets.get(arguments).ok_or_else(|| ParseError::new(self.index, source, format!("bare WITH {arguments} requires a preceding LET {arguments} = value")))?;
                            if values.len() != 1 {
                                return Err(ParseError::new(
                                    self.index,
                                    source,
                                    format!("bare WITH {arguments} requires a scalar LET value"),
                                ));
                            }
                            (arguments, Template::literal(values[0].clone()))
                        }
                        _ => {
                            return Err(ParseError::new(
                                self.index,
                                source,
                                "WITH requires NAME=value, bare NAME, or UNSAFE IGNORE <path>",
                            ))
                        }
                    };
                    if environment.insert(name.to_owned(), value).is_some() {
                        return Err(ParseError::new(
                            self.index,
                            source,
                            format!("WITH {name} is duplicated for this node"),
                        ));
                    }
                }
                "EXPECT" => {
                    self.index += 1;
                    if expected.replace(arguments.to_owned()).is_some() || arguments.is_empty() {
                        return Err(ParseError::new(
                            self.index,
                            source,
                            "EXPECT requires exactly one hash and may appear once per FETCH node",
                        ));
                    }
                }
                _ => break,
            }
        }
        let _ = (node_line, node_source);
        Ok((environment, ignored, expected))
    }

    pub(super) fn declare_name(
        &mut self,
        name: &str,
        kind: &'static str,
        line: usize,
        source: &str,
    ) -> Result<(), ParseError> {
        if let Some(first) = self.names.get(name) {
            return Err(ParseError::new(
              line,
              source,
              format!(
                  "name {name:?} is already bound by a {} on line {}; block and binder names share one namespace, so rename this declaration",
                  first.kind, first.line
              ),
          ));
        }
        self.names
            .insert(name.to_owned(), DeclaredName { kind, line });
        Ok(())
    }
}
