use crate::*;

use super::super::machine::{CurrentBlock, ParseError, Parser};
use super::super::validate::*;

impl Parser<'_> {
    pub(in crate::parser) fn import(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        let fields = at_least_one_field(arguments, line, source, "IMPORT <pkg-ref>…")?;
        let block = self.current.clone().ok_or_else(|| {
            ParseError::new(
                line,
                source,
                "IMPORT is outside a block; put it inside BUILDER, SERVICE, APP, or ITEM",
            )
        })?;
        let existing = match &block {
            CurrentBlock::Builder(name) => &self.builders[name].imports,
            CurrentBlock::Artifact(name) => &self.artifacts[name].imports,
        };
        let mut additions = Vec::new();
        for field in fields {
            reject_runtime_variable(field, "IMPORT package", line, source)?;
            let package = self.build_template(field, line, source, false)?;
            validate_import_template(&package, &self.inputs, line, source)?;
            if existing
                .iter()
                .chain(&additions)
                .any(|candidate| candidate.same_value(&package))
            {
                return Err(ParseError::new(
                    line,
                    source,
                    format!("IMPORT package {field:?} is duplicated"),
                ));
            }
            additions.push(package);
        }
        match block {
            CurrentBlock::Builder(name) => self
                .builders
                .get_mut(&name)
                .expect("builder exists")
                .imports
                .extend(additions),
            CurrentBlock::Artifact(name) => self
                .artifacts
                .get_mut(&name)
                .expect("artifact exists")
                .imports
                .extend(additions),
        }
        Ok(())
    }

    pub(in crate::parser) fn copy(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        if arguments.starts_with("--from=") {
            return Err(ParseError::new(
              line,
              source,
              "COPY --from is Docker vocabulary; use a named binder such as COPY ${build}/<path> /<destination>; see docs/migrate.md#docker-vocabulary",
          ));
        }
        let fields = exact_fields(arguments, 2, line, source, "COPY <src> <dst>")?;
        if fields[0].starts_with('/') && fields[1].contains("${") {
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "COPY arguments are source then destination; write COPY {} {}",
                    fields[1], fields[0]
                ),
            ));
        }
        reject_runtime_variable(fields[0], "COPY source", line, source)?;
        reject_runtime_variable(fields[1], "COPY destination", line, source)?;
        let src = self.build_template(fields[0], line, source, false)?;
        validate_copy_source(&src, line, source)?;
        let block = self.current.clone().ok_or_else(|| {
            ParseError::new(
                line,
                source,
                "COPY is outside a block; put it inside BUILDER, SERVICE, APP, or ITEM",
            )
        })?;
        let destination = match &block {
            CurrentBlock::Builder(_) => {
                if fields[1].starts_with('/') {
                    let replacement = fields[1]
                        .strip_prefix('/')
                        .filter(|path| !path.is_empty())
                        .unwrap_or(".");
                    return Err(ParseError::new(
                        line,
                        source,
                        format!(
                          "BUILDER COPY destination must be workdir-relative; write {replacement}"
                      ),
                    ));
                }
                validate_copy_relative_path(fields[1], "COPY destination", line, source)?;
                fields[1]
            }
            CurrentBlock::Artifact(_) => {
                normalize_artifact_copy_destination(fields[1], "COPY destination", line, source)?
            }
        };
        let name = match &block {
            CurrentBlock::Builder(name) | CurrentBlock::Artifact(name) => name,
        };
        let duplicate = !self
            .destinations
            .get_mut(name)
            .expect("block destinations exist")
            .insert(destination.to_owned());
        if duplicate && matches!(&block, CurrentBlock::Artifact(_)) {
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "{} destination {:?} is already populated",
                    self.names[name].kind, destination
                ),
            ));
        }
        let mode = match &block {
            CurrentBlock::Builder(_) => CopyMode::Materialize,
            CurrentBlock::Artifact(_) => self.artifact_copy_mode(&src),
        };
        let copy = Copy {
            src,
            dst: destination.to_owned(),
            mode,
            line,
            source: source.to_owned(),
        };
        match block {
            CurrentBlock::Builder(name) => self
                .builders
                .get_mut(&name)
                .expect("builder exists")
                .steps
                .push(BuildStep::Copy(copy)),
            CurrentBlock::Artifact(name) => self
                .artifacts
                .get_mut(&name)
                .expect("artifact exists")
                .copies
                .push(copy),
        }
        Ok(())
    }

    pub(in crate::parser) fn heredoc(
        &mut self,
        directive: &str,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        let artifact_name = self
            .current_artifact_name(directive, line, source)?
            .to_owned();
        let fields = exact_fields(
            arguments,
            2,
            line,
            source,
            &format!("{directive} <dst> <<EOF"),
        )?;
        let destination = normalize_artifact_destination(
            fields[0],
            &format!("{directive} destination"),
            line,
            source,
        )?;
        reject_build_interpolation(fields[0], &format!("{directive} destination"), line, source)?;
        reject_runtime_variable(fields[0], &format!("{directive} destination"), line, source)?;
        let delimiter = fields[1]
            .strip_prefix("<<")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                ParseError::new(
                    line,
                    source,
                    format!("{directive} heredoc must use << followed by a delimiter"),
                )
            })?;
        let contents = self.read_heredoc_body(directive, delimiter, line, source)?;
        self.claim_artifact_destination(destination, line, source)?;
        let assembly = Assembly::File {
            dst: destination.to_owned(),
            contents,
            line,
        };
        self.artifacts
            .get_mut(&artifact_name)
            .expect("artifact exists")
            .assembly
            .push(assembly);
        Ok(())
    }

    pub(in crate::parser) fn read_heredoc_body(
        &mut self,
        directive: &str,
        delimiter: &str,
        line: usize,
        source: &str,
    ) -> Result<Template, ParseError> {
        let mut contents = Template { parts: Vec::new() };
        while self.index < self.lines.len() {
            let body_line_number = self.index + 1;
            let body_line = self.lines[self.index];
            self.index += 1;
            if body_line == delimiter {
                return Ok(contents);
            }
            append_template(
                &mut contents,
                self.build_template(body_line, body_line_number, body_line, true)?,
            );
            push_literal(&mut contents, "\n");
        }
        Err(ParseError::new(
          line,
          source,
          format!(
              "unterminated {directive} heredoc; close it with {delimiter} on a line by itself with no indentation"
          ),
      ))
    }

    fn artifact_copy_mode(&self, source: &Template) -> CopyMode {
        match source.parts.first() {
            Some(TemplatePart::Package { .. }) => CopyMode::Link,
            Some(TemplatePart::Literal(path)) if path.starts_with("/nix/store/") => CopyMode::Link,
            Some(TemplatePart::Binder { name, .. })
                if self.builders.contains_key(name) || self.fetches.contains_key(name) =>
            {
                CopyMode::LinkNormalized
            }
            Some(TemplatePart::Binder { name, .. })
                if self
                    .inputs
                    .get(name)
                    .is_some_and(|input| input.kind == InputKind::Artifact) =>
            {
                CopyMode::Link
            }
            _ => CopyMode::Materialize,
        }
    }
}
