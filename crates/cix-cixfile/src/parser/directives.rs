//! Directive handlers mutate the parser machine after lexical dispatch.

use std::collections::BTreeSet;

use crate::*;

use super::machine::{CurrentBlock, DeclaredName, ParseError, Parser, ServiceMetadata};
use super::validate::*;

impl Parser<'_> {
    pub(super) fn from(
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

    pub(super) fn fetch(
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

    pub(super) fn begin_builder(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        let fields = exact_fields(arguments, 1, line, source, "BUILDER <name>")?;
        let name = fields[0];
        validate_name("builder", name, line, source)?;
        self.declare_name(name, "BUILDER block", line, source)?;
        self.builders.insert(name.to_owned(), Builder::empty(line));
        self.builder_order.push(name.to_owned());
        self.destinations.insert(name.to_owned(), BTreeSet::new());
        self.current = Some(CurrentBlock::Builder(name.to_owned()));
        Ok(())
    }

    pub(super) fn begin_artifact(
        &mut self,
        kind: ArtifactKind,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        let fields = exact_fields(
            arguments,
            1,
            line,
            source,
            &format!("{} <name>", kind.keyword()),
        )?;
        let name = fields[0];
        validate_name("artifact", name, line, source)?;
        self.declare_name(name, "artifact block", line, source)?;
        self.artifacts
            .insert(name.to_owned(), Artifact::empty(kind, line));
        self.artifact_order.push(name.to_owned());
        self.destinations.insert(name.to_owned(), BTreeSet::new());
        self.metadata
            .insert(name.to_owned(), ServiceMetadata::default());
        self.current = Some(CurrentBlock::Artifact(name.to_owned()));
        Ok(())
    }

    pub(super) fn import(
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

    pub(super) fn copy(
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

    pub(super) fn run(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        let Some(CurrentBlock::Builder(name)) = self.current.clone() else {
            let message = if arguments.split_whitespace().next() == Some("apt-get") {
                "RUN apt-get is Docker vocabulary; use IMPORT ${pkgs.<package>} in a BUILDER, then RUN the imported tools; see docs/migrate.md#docker-vocabulary"
            } else {
                "RUN is outside a BUILDER; add BUILDER <name> before this line"
            };
            return Err(ParseError::new(line, source, message));
        };
        let command = if let Some(delimiter) = heredoc_delimiter(arguments, "RUN", line, source)? {
            self.read_heredoc_body("RUN", delimiter, line, source)?
        } else {
            if arguments.is_empty() {
                return Err(ParseError::new(line, source, "RUN requires a command"));
            }
            self.build_template(arguments, line, source, false)?
        };
        self.push_builder_command(&name, None, false, line, source, command);
        Ok(())
    }

    pub(super) fn builder_command(
        &mut self,
        builder: &str,
        fetch: bool,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        let directive = if fetch { "FETCH" } else { "RUN" };
        if arguments.is_empty() {
            return Err(ParseError::new(
                line,
                source,
                format!("{directive} requires a command"),
            ));
        }
        let (expected, command) = if fetch {
            parse_fetch_expect(arguments, line, source)?
        } else {
            (None, arguments)
        };
        let command = self.build_template(command, line, source, false)?;
        self.push_builder_command(builder, expected, fetch, line, source, command);
        Ok(())
    }

    pub(super) fn push_builder_command(
        &mut self,
        builder: &str,
        expected: Option<String>,
        fetch: bool,
        line: usize,
        source: &str,
        command: Template,
    ) {
        let step = if fetch {
            BuildStep::Fetch {
                expected,
                command,
                line,
                source: source.to_owned(),
            }
        } else {
            BuildStep::Run {
                command,
                line,
                source: source.to_owned(),
            }
        };
        self.builders
            .get_mut(builder)
            .expect("builder exists")
            .steps
            .push(step);
    }

    pub(super) fn heredoc(
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

    pub(super) fn read_heredoc_body(
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

    pub(super) fn link(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        let fields = exact_fields(arguments, 2, line, source, "LINK <target> <linkpath>")?;
        let unmistakably_old_order = (fields[1].contains("${")
            || fields[1].starts_with("/nix/store/"))
            && validate_item_path(fields[0], "LINK path", line, source).is_ok();
        if unmistakably_old_order
            || validate_item_path(fields[1], "LINK path", line, source).is_err()
                && validate_item_path(fields[0], "LINK path", line, source).is_ok()
        {
            return Err(ParseError::new(
              line,
              source,
              "LINK arguments are target then link path; write LINK <target> <absolute-linkpath>; see docs/cixfile.md#link",
          ));
        }
        let destination = normalize_artifact_destination(fields[1], "LINK path", line, source)?;
        reject_build_interpolation(fields[1], "LINK path", line, source)?;
        reject_runtime_variable(fields[1], "LINK path", line, source)?;
        reject_runtime_variable(fields[0], "LINK target", line, source)?;
        let target = self.build_template(fields[0], line, source, false)?;
        if target.is_empty() {
            return Err(ParseError::new(
                line,
                source,
                "LINK target must not be empty",
            ));
        }
        validate_copy_source(&target, line, source)?;
        let mode = self.artifact_copy_mode(&target);
        self.claim_artifact_destination(destination, line, source)?;
        eprintln!(
            "warning: line {line}: LINK is deprecated; use COPY {} {}",
            fields[0], fields[1]
        );
        self.current_artifact_mut("LINK", line, source)?
            .copies
            .push(Copy {
                src: target,
                dst: destination.to_owned(),
                mode,
                line,
                source: source.to_owned(),
            });
        Ok(())
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

    pub(super) fn start(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
        start_pre: bool,
    ) -> Result<(), ParseError> {
        let directive = if start_pre { "START_PRE" } else { "START" };
        let fields = argv_fields(arguments, line, source, directive)?;
        validate_artifact_command_path(&fields[0], directive, line, source)?;
        let artifact_name = self
            .current_artifact_name(directive, line, source)?
            .to_owned();
        let kind = self.artifacts[&artifact_name].kind;
        if !kind.is_runnable() {
            return Err(self.item_seam_parse_error(directive, line, source));
        }
        if kind == ArtifactKind::App && start_pre {
            return Err(ParseError::new(
              line,
              source,
              "START_PRE is not allowed inside APP; move preparation into the APP executable; see docs/cixfile.md#artifact-kinds",
          ));
        }
        let templates = fields
            .iter()
            .map(|field| self.build_template(field, line, source, false))
            .collect::<Result<Vec<_>, _>>()?;
        let service = &mut self
            .artifacts
            .get_mut(&artifact_name)
            .expect("artifact exists")
            .service;
        if start_pre {
            if service.start_pre.is_some() {
                return Err(ParseError::new(
                    line,
                    source,
                    "START_PRE is already declared for this service",
                ));
            }
            service.start_pre = Some(templates);
            service.start_pre_line = Some(line);
            self.metadata
                .get_mut(&artifact_name)
                .expect("artifact metadata exists")
                .start_pre = Some((line, source.to_owned()));
        } else {
            if !service.start.is_empty() {
                return Err(ParseError::new(
                    line,
                    source,
                    "START is already declared for this artifact; remove one START line",
                ));
            }
            service.start = templates;
            service.start_line = line;
            self.metadata
                .get_mut(&artifact_name)
                .expect("artifact metadata exists")
                .start = Some((line, source.to_owned()));
        }
        Ok(())
    }

    pub(super) fn env(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        let fields = argv_fields(arguments, line, source, "ENV")?;
        if fields.get(1).is_some_and(|field| field == "=") {
            return Err(ParseError::new(
                line,
                source,
                "ENV assignments do not allow spaces around '='; write `ENV NAME=value`",
            ));
        }
        let assignment = fields[0].split_once('=');
        let name = assignment.map_or(fields[0].as_str(), |(name, _)| name);
        validate_env_name(name, line, source)?;
        if matches!(self.current, Some(CurrentBlock::Builder(_))) {
            let Some((_, value)) = assignment else {
                return Err(ParseError::new(
                    line,
                    source,
                    "builder ENV defaults must use `NAME=value`",
                ));
            };
            if fields.len() != 1 {
                return Err(ParseError::new(
                    line,
                    source,
                    "expected builder ENV NAME=value",
                ));
            }
            let builder_name = self.current_builder_name("ENV", line, source)?.to_owned();
            let value = self.build_template(value, line, source, false)?;
            self.builders
                .get_mut(&builder_name)
                .expect("current builder exists")
                .steps
                .push(BuildStep::Env {
                    name: name.to_owned(),
                    value,
                    line,
                    source: source.to_owned(),
                });
            return Ok(());
        }
        self.require_artifact_kind(
            "ENV",
            line,
            source,
            &[ArtifactKind::Service, ArtifactKind::App],
        )?;
        let index = 1;
        let default = if let Some((_, value)) = assignment {
            reject_runtime_variable(value, "ENV default", line, source)?;
            Some(self.build_template(value, line, source, false)?)
        } else {
            None
        };
        let mut required = false;
        let mut secret = false;
        for flag in &fields[index..] {
            match flag.as_str() {
                "required" if !required => required = true,
                "secret" if !secret => secret = true,
                "required" | "secret" => {
                    return Err(ParseError::new(
                        line,
                        source,
                        format!("ENV flag {flag:?} is duplicated"),
                    ));
                }
                _ => {
                    return Err(ParseError::new(
                        line,
                        source,
                        format!("unknown ENV field {flag:?}"),
                    ));
                }
            }
        }
        if required && default.is_some() {
            return Err(ParseError::new(
                line,
                source,
                "ENV required forbids a default; write either `ENV NAME=value` or `ENV NAME required`",
            ));
        }
        let artifact_name = self.current_artifact_name("ENV", line, source)?.to_owned();
        let service = &mut self
            .artifacts
            .get_mut(&artifact_name)
            .expect("artifact exists")
            .service;
        if service.env.contains_key(name) {
            return Err(ParseError::new(
                line,
                source,
                format!("ENV {name:?} is already declared"),
            ));
        }
        service.env.insert(
            name.to_owned(),
            Env {
                default,
                required,
                secret,
            },
        );
        Ok(())
    }

    pub(super) fn secret(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        self.require_artifact_kind(
            "SECRET",
            line,
            source,
            &[ArtifactKind::Service, ArtifactKind::App],
        )?;
        let fields = arguments.split_whitespace().collect::<Vec<_>>();
        let (name, as_env) = match fields.as_slice() {
            [name] => (*name, None),
            [name, "AS", environment] => {
                validate_env_name(environment, line, source)?;
                if !environment.ends_with("_FILE") {
                    return Err(ParseError::new(
                        line,
                        source,
                        "SECRET AS variable must end in _FILE; it receives a credential path, never a secret value",
                    ));
                }
                (*name, Some((*environment).to_owned()))
            }
            _ => {
                return Err(ParseError::new(
                    line,
                    source,
                    "SECRET syntax is SECRET <name> [AS <VAR_FILE>]",
                ));
            }
        };
        validate_name("secret", name, line, source)?;
        let artifact_name = self
            .current_artifact_name("SECRET", line, source)?
            .to_owned();
        let service = &mut self
            .artifacts
            .get_mut(&artifact_name)
            .expect("current artifact exists")
            .service;
        if service.secrets.contains_key(name) {
            return Err(ParseError::new(
                line,
                source,
                format!("SECRET {name:?} is already declared"),
            ));
        }
        service.secrets.insert(name.to_owned(), Secret { as_env });
        Ok(())
    }

    pub(super) fn port(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        self.require_artifact_kind("PORT", line, source, &[ArtifactKind::Service])?;
        let fields = exact_fields(
            arguments,
            3,
            line,
            source,
            "PORT <name> = [udp:]<$VAR|value>",
        )?;
        validate_name("port", fields[0], line, source)?;
        if fields[1] != "=" {
            return Err(ParseError::new(
                line,
                source,
                "PORT name must be followed by '='",
            ));
        }
        let (protocol, value) = if let Some(value) = fields[2].strip_prefix("udp:") {
            (Protocol::Udp, value)
        } else if let Some((port, protocol)) = fields[2].rsplit_once('/') {
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "PORT uses systemd protocol spelling; write udp:{port} instead of {port}/{protocol}"
                ),
            ));
        } else if fields[2].contains(':') {
            return Err(ParseError::new(
                line,
                source,
                "PORT supports bare TCP ports or udp:<port>; SCTP is not supported",
            ));
        } else {
            (Protocol::Tcp, fields[2])
        };
        let port_source = if let Some(variable) = value.strip_prefix('$') {
            validate_env_name(variable, line, source)?;
            PortSource::Env(variable.to_owned())
        } else {
            let value = value.parse::<u16>().map_err(|_| {
                ParseError::new(line, source, "PORT value must be between 1 and 65535")
            })?;
            if value == 0 {
                return Err(ParseError::new(
                    line,
                    source,
                    "PORT value must be between 1 and 65535",
                ));
            }
            PortSource::Value(value)
        };
        let port = Port {
            source: port_source,
            protocol,
        };
        let name = self.current_artifact_name("PORT", line, source)?.to_owned();
        let service = &mut self
            .artifacts
            .get_mut(&name)
            .expect("artifact exists")
            .service;
        if service.listeners.contains(fields[0]) {
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "PORT {:?} conflicts with a LISTENER of the same name",
                    fields[0]
                ),
            ));
        }
        if service.ports.contains_key(fields[0]) {
            return Err(ParseError::new(
                line,
                source,
                format!("PORT {:?} is already declared", fields[0]),
            ));
        }
        service.ports.insert(fields[0].to_owned(), port);
        self.metadata
            .get_mut(&name)
            .expect("artifact metadata exists")
            .ports
            .insert(fields[0].to_owned(), (line, source.to_owned()));
        Ok(())
    }

    pub(super) fn listener(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        self.require_artifact_kind("LISTENER", line, source, &[ArtifactKind::Service])?;
        let fields = exact_fields(arguments, 1, line, source, "LISTENER <name>")?;
        validate_name("listener", fields[0], line, source)?;
        let service = self.current_service_mut("LISTENER", line, source)?;
        if service.ports.contains_key(fields[0]) {
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "LISTENER {:?} conflicts with a PORT of the same name",
                    fields[0]
                ),
            ));
        }
        if !service.listeners.insert(fields[0].to_owned()) {
            return Err(ParseError::new(
                line,
                source,
                format!("LISTENER {:?} is already declared", fields[0]),
            ));
        }
        Ok(())
    }

    pub(super) fn health_probe(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
        readiness: bool,
    ) -> Result<(), ParseError> {
        let directive = if readiness { "READINESS" } else { "LIVENESS" };
        self.require_artifact_kind(
            directive,
            line,
            source,
            &[ArtifactKind::Service, ArtifactKind::App],
        )?;
        let parameter = if readiness { "IN" } else { "EVERY" };
        let fields = arguments.split_whitespace().collect::<Vec<_>>();
        let (probe, marker, duration) = match fields.as_slice() {
            ["notify", marker, duration] => (Probe::Notify, *marker, *duration),
            ["http", target, marker, duration] => {
                validate_http_probe_target(target, line, source)?;
                (Probe::Http((*target).to_owned()), *marker, *duration)
            }
            ["tcp", target, marker, duration] => {
                validate_tcp_probe_target(target, line, source)?;
                (Probe::Tcp((*target).to_owned()), *marker, *duration)
            }
            _ => {
                return Err(ParseError::new(
                    line,
                    source,
                    format!(
                        "{directive} requires `http <host:port/path> {parameter} <duration>`, `tcp <host:port> {parameter} <duration>`, or `notify {parameter} <duration>`"
                    ),
                ))
            }
        };
        if marker != parameter {
            return Err(ParseError::new(
                line,
                source,
                format!("{directive} uses {parameter} before its duration, not {marker}"),
            ));
        }
        validate_probe_duration(duration, line, source)?;
        let service = self.current_service_mut(directive, line, source)?;
        if readiness {
            if service.readiness.is_some() {
                return Err(ParseError::new(
                    line,
                    source,
                    "READINESS is already declared for this artifact",
                ));
            }
            service.readiness = Some(Readiness {
                probe,
                timeout: duration.to_owned(),
            });
        } else {
            if service.liveness.is_some() {
                return Err(ParseError::new(
                    line,
                    source,
                    "LIVENESS is already declared for this artifact",
                ));
            }
            service.liveness = Some(Liveness {
                probe,
                interval: duration.to_owned(),
            });
        }
        Ok(())
    }

    pub(super) fn directory(
        &mut self,
        directive: &str,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        let allowed = match directive {
            "STATEDIR" | "CACHEDIR" => &[ArtifactKind::Service, ArtifactKind::App][..],
            _ => &[ArtifactKind::Service][..],
        };
        self.require_artifact_kind(directive, line, source, allowed)?;
        let fields = exact_fields(arguments, 1, line, source, &format!("{directive} <path>"))?;
        let (path, ro) = parse_directory_declaration(directive, fields[0], line, source)?;
        let service = self.current_service_mut(directive, line, source)?;
        if service.dirs.state.contains(path)
            || service.dirs.cache.contains(path)
            || service.dirs.logs.contains(path)
            || service.dirs.config.contains(path)
            || service.dirs.run.contains(path)
            || service.dirs.data.contains_key(path)
        {
            return Err(ParseError::new(
                line,
                source,
                format!("{directive} path {path:?} is duplicated"),
            ));
        }
        if directive == "DIR" {
            service.dirs.data.insert(path.to_owned(), ro);
            return Ok(());
        }
        let paths = match directive {
            "STATEDIR" => &mut service.dirs.state,
            "CACHEDIR" => &mut service.dirs.cache,
            "LOGDIR" => &mut service.dirs.logs,
            "CONFIGDIR" => &mut service.dirs.config,
            "RUNDIR" => &mut service.dirs.run,
            _ => unreachable!(),
        };
        paths.insert(path.to_owned());
        Ok(())
    }

    pub(super) fn claim(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        self.require_artifact_kind(
            "CLAIM",
            line,
            source,
            &[ArtifactKind::Service, ArtifactKind::App],
        )?;
        let fields = arguments.split_whitespace().collect::<Vec<_>>();
        let claim = match fields.as_slice() {
            ["jit" | "egress" | "gpu"] => Claim::Named(fields[0].to_owned()),
            ["device", device] => {
                Self::validate_device_path(device, line, source)?;
                Claim::Device((*device).to_owned())
            }
            _ => {
                return Err(ParseError::new(
                    line,
                    source,
                    "CLAIM requires one of: jit, egress, gpu, or device /dev/<node>",
                ))
            }
        };
        let service = self.current_service_mut("CLAIM", line, source)?;
        if !service.claims.insert(claim.clone()) {
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "CLAIM {:?} is already declared for this artifact",
                    match claim {
                        Claim::Named(name) => name,
                        Claim::Device(path) => format!("device {path}"),
                    }
                ),
            ));
        }
        Ok(())
    }

    pub(super) fn shm(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        self.require_artifact_kind(
            "SHM",
            line,
            source,
            &[ArtifactKind::Service, ArtifactKind::App],
        )?;
        let fields = exact_fields(arguments, 1, line, source, "SHM <size>")?;
        Self::validate_systemd_size(fields[0], line, source)?;
        let service = self.current_service_mut("SHM", line, source)?;
        if service.shm.replace(fields[0].to_owned()).is_some() {
            return Err(ParseError::new(
                line,
                source,
                "SHM is already declared for this artifact",
            ));
        }
        Ok(())
    }

    fn validate_device_path(path: &str, line: usize, source: &str) -> Result<(), ParseError> {
        let path = std::path::Path::new(path);
        if !path.is_absolute()
            || path == std::path::Path::new("/dev")
            || !path.starts_with("/dev")
            || path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::CurDir | std::path::Component::ParentDir
                )
            })
        {
            return Err(ParseError::new(
                line,
                source,
                "CLAIM device requires a clean absolute path under /dev",
            ));
        }
        Ok(())
    }

    fn validate_systemd_size(size: &str, line: usize, source: &str) -> Result<(), ParseError> {
        let digits = size.bytes().take_while(u8::is_ascii_digit).count();
        let suffix = size.get(digits..).unwrap_or_default().to_ascii_uppercase();
        let valid = digits > 0
            && matches!(
                suffix.as_str(),
                "" | "B"
                    | "K"
                    | "KB"
                    | "KIB"
                    | "M"
                    | "MB"
                    | "MIB"
                    | "G"
                    | "GB"
                    | "GIB"
                    | "T"
                    | "TB"
                    | "TIB"
                    | "P"
                    | "PB"
                    | "PIB"
                    | "E"
                    | "EB"
                    | "EIB"
            );
        if !valid {
            return Err(ParseError::new(
                line,
                source,
                "SHM size must use systemd size syntax, for example 64M or 1G",
            ));
        }
        Ok(())
    }

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

    pub(super) fn current_artifact_mut(
        &mut self,
        directive: &str,
        line: usize,
        source: &str,
    ) -> Result<&mut Artifact, ParseError> {
        let name = self
            .current_artifact_name(directive, line, source)?
            .to_owned();
        Ok(self
            .artifacts
            .get_mut(&name)
            .expect("current artifact exists"))
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
        let template = build_template(input, line, source, heredoc, &self.inputs, &self.names)?;
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
