use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path};

use crate::model::{
    Artifact, ArtifactKind, Assembly, BuildStep, Builder, Cixfile, Copy, Env, Fetch, Input,
    InputKind, Port, Service, Template, TemplatePart,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub source: String,
    pub message: String,
}

impl ParseError {
    fn new(line: usize, source: &str, message: impl Into<String>) -> Self {
        Self {
            line,
            source: source.to_owned(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "line {}: {}\n  | {:?}",
            self.line, self.message, self.source
        )
    }
}

impl std::error::Error for ParseError {}

struct Parser<'a> {
    lines: Vec<&'a str>,
    index: usize,
    inputs: BTreeMap<String, Input>,
    fetches: BTreeMap<String, Fetch>,
    fetch_order: Vec<String>,
    builders: BTreeMap<String, Builder>,
    builder_order: Vec<String>,
    artifacts: BTreeMap<String, Artifact>,
    artifact_order: Vec<String>,
    names: BTreeMap<String, DeclaredName>,
    destinations: BTreeMap<String, BTreeSet<String>>,
    metadata: BTreeMap<String, ServiceMetadata>,
    current: Option<CurrentBlock>,
}

#[derive(Clone)]
struct DeclaredName {
    kind: &'static str,
    line: usize,
}

#[derive(Clone)]
enum CurrentBlock {
    Builder(String),
    Artifact(String),
}

#[derive(Default)]
struct ServiceMetadata {
    exec: Option<(usize, String)>,
    setup: Option<(usize, String)>,
    ports: BTreeMap<String, (usize, String)>,
}

pub fn parse(input: &str) -> Result<Cixfile, ParseError> {
    Parser {
        lines: input.lines().collect(),
        index: 0,
        inputs: BTreeMap::new(),
        fetches: BTreeMap::new(),
        fetch_order: Vec::new(),
        builders: BTreeMap::new(),
        builder_order: Vec::new(),
        artifacts: BTreeMap::new(),
        artifact_order: Vec::new(),
        names: BTreeMap::new(),
        destinations: BTreeMap::new(),
        metadata: BTreeMap::new(),
        current: None,
    }
    .parse()
}

impl Parser<'_> {
    fn parse(mut self) -> Result<Cixfile, ParseError> {
        while self.index < self.lines.len() {
            let line_number = self.index + 1;
            let source = self.lines[self.index];
            self.index += 1;
            let initial = source.trim();
            if initial.is_empty() || initial.starts_with('#') {
                continue;
            }
            let mut logical = source.trim_end().to_owned();
            while logical.ends_with('\\') {
                logical.pop();
                logical.truncate(logical.trim_end().len());
                let continuation_line = self.index + 1;
                let Some(continuation) = self.lines.get(self.index).copied() else {
                    return Err(ParseError::new(
                        line_number,
                        source,
                        "directive line continuation has no following physical line",
                    ));
                };
                self.index += 1;
                let fragment = continuation
                    .trim()
                    .strip_suffix('\\')
                    .unwrap_or(continuation.trim())
                    .trim_end();
                if fragment.contains("${") {
                    self.build_template(fragment, continuation_line, continuation, false)?;
                }
                logical.push(' ');
                logical.push_str(continuation.trim());
                if continuation.trim_end().ends_with('\\') && self.index == self.lines.len() {
                    return Err(ParseError::new(
                        continuation_line,
                        continuation,
                        "directive line continuation has no following physical line",
                    ));
                }
            }
            let trimmed = logical.trim();
            let (directive, arguments) = trimmed
                .split_once(char::is_whitespace)
                .map_or((trimmed, ""), |(directive, arguments)| {
                    (directive, arguments.trim())
                });
            if let Some(error) = self.item_seam_error(directive, line_number, source) {
                return Err(error);
            }
            match directive {
                "FROM" => self.from(line_number, source, arguments)?,
                "FETCH" => self.fetch(line_number, source, arguments)?,
                "BUILDER" => self.begin_builder(line_number, source, arguments)?,
                "SERVICE" => {
                    self.begin_artifact(ArtifactKind::Service, line_number, source, arguments)?
                }
                "APP" => self.begin_artifact(ArtifactKind::App, line_number, source, arguments)?,
                "ITEM" => self.begin_artifact(ArtifactKind::Item, line_number, source, arguments)?,
                "COPY" => self.copy(line_number, source, arguments)?,
                "RUN" => self.run(line_number, source, arguments)?,
                "IMPORT" => self.import(line_number, source, arguments)?,
                "PATH" => {
                    let message = match &self.current {
                        Some(CurrentBlock::Builder(_)) => "PATH was replaced by IMPORT (D58)",
                        Some(CurrentBlock::Artifact(_)) => {
                            "PATH was removed (D58); declare ENV PATH = ... explicitly"
                        }
                        None => "PATH was replaced by IMPORT (D58)",
                    };
                    return Err(ParseError::new(line_number, source, message));
                }
                "CACHE" => {
                    return Err(ParseError::new(
                        line_number,
                        source,
                        "CACHE was removed (D57): workspaces persist by default and nothing is keyed unless read; delete the line",
                    ));
                }
                "FILE" => self.heredoc(directive, line_number, source, arguments)?,
                "SCRIPT" => {
                    return Err(ParseError::new(
                        line_number,
                        source,
                        "SCRIPT was dropped (D55); COPY a real script and EXEC ${pkgs.bash}/bin/sh <path>, or use FILE if the content needs store-path interpolation",
                    ));
                }
                "LINK" => self.link(line_number, source, arguments)?,
                "EXEC" => self.exec(line_number, source, arguments, false)?,
                "SETUP" => self.exec(line_number, source, arguments, true)?,
                "ENV" => self.env(line_number, source, arguments)?,
                "PORT" => self.port(line_number, source, arguments)?,
                "LISTENER" => self.listener(line_number, source, arguments)?,
                "STATEDIR" | "CACHEDIR" | "LOGSDIR" | "CONFIGDIR" | "RUNDIR" => {
                    self.directory(directive, line_number, source, arguments)?
                }
                "STATE" => return Err(ParseError::new(line_number, source, "STATE was renamed to STATEDIR by D52; replace this directive with STATEDIR")),
                "LOGS" => return Err(ParseError::new(line_number, source, "LOGS was renamed to LOGSDIR by D52; replace this directive with LOGSDIR")),
                "CONFIG" => return Err(ParseError::new(line_number, source, "CONFIG was renamed to CONFIGDIR by D52; replace this directive with CONFIGDIR")),
                "GRANT" => self.grant(line_number, source, arguments)?,
                "JIT" => return Err(ParseError::new(line_number, source, "JIT was replaced by GRANT jit (D60); replace this directive with GRANT jit")),
                "EGRESS" => return Err(ParseError::new(line_number, source, "EGRESS was replaced by GRANT egress (D60); replace this directive with GRANT egress")),
                "OUTBOUND" => {
                    return Err(ParseError::new(
                        line_number,
                        source,
                        "OUTBOUND was replaced by GRANT egress (D60); replace this directive with GRANT egress",
                    ));
                }
                "TAKE" => return Err(take_removed_error(line_number, source, arguments)),
                "PKG" => return Err(pkg_removed_error(line_number, source, arguments)),
                _ => {
                    return Err(ParseError::new(
                        line_number,
                        source,
                        format!("unknown directive {directive:?}"),
                    ));
                }
            }
        }

        let first = || {
            self.lines
                .iter()
                .enumerate()
                .find(|(_, line)| !line.trim().is_empty() && !line.trim().starts_with('#'))
                .map_or((1, ""), |(line, source)| (line + 1, *source))
        };
        if self
            .inputs
            .values()
            .all(|input| input.kind != InputKind::PackageUniverse)
        {
            let (line, source) = first();
            return Err(ParseError::new(
                line,
                source,
                "a Cixfile needs a package universe; try: FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs",
            ));
        }
        if self.artifacts.is_empty() {
            let (line, source) = first();
            return Err(ParseError::new(
                line,
                source,
                "a Cixfile must declare at least one SERVICE, APP, or ITEM block",
            ));
        }
        for (name, artifact) in &self.artifacts {
            if artifact.kind.is_runnable() && artifact.service.exec.is_empty() {
                return Err(ParseError::new(
                    artifact.line,
                    self.lines
                        .get(artifact.line - 1)
                        .copied()
                        .unwrap_or_default(),
                    format!(
                        "{} {name:?} must declare exactly one EXEC",
                        artifact.kind.keyword()
                    ),
                ));
            }
            if artifact.kind.is_runnable() {
                validate_service_references(&artifact.service, &self.metadata[name])?;
            }
        }
        Ok(Cixfile {
            inputs: self.inputs,
            fetches: self.fetches,
            fetch_order: self.fetch_order,
            builders: self.builders,
            builder_order: self.builder_order,
            artifacts: self.artifacts,
            artifact_order: self.artifact_order,
        })
    }

    fn from(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        if self.current.is_some() {
            return Err(ParseError::new(
                line,
                source,
                "FROM is a prelude declaration and must appear before the first block",
            ));
        }
        let fields = arguments.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 3 || fields.get(1) != Some(&"AS") {
            return Err(ParseError::new(
                line,
                source,
                "FROM requires an explicit binder: FROM <flakeref|index-ref:tag> AS <name>",
            ));
        }
        let (url, kind) = normalize_input(fields[0], line, source)?;
        let name = fields[2];
        validate_namespace(name, line, source)?;
        self.declare_name(name, "FROM binder", line, source)?;
        self.inputs
            .insert(name.to_owned(), Input { url, kind, line });
        Ok(())
    }

    fn fetch(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
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
                "top-level FETCH requires a binder and command: FETCH <name> [EXPECT <sri-hash>] <command…>",
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

    fn begin_builder(
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

    fn begin_artifact(
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

    fn import(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        let fields = at_least_one_field(arguments, line, source, "IMPORT <pkg-ref>…")?;
        let Some(CurrentBlock::Builder(name)) = self.current.clone() else {
            return Err(ParseError::new(
                line,
                source,
                "IMPORT is only legal inside a BUILDER block",
            ));
        };
        let existing = &self.builders[&name].imports;
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
        self.builders
            .get_mut(&name)
            .expect("builder exists")
            .imports
            .extend(additions);
        Ok(())
    }

    fn copy(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        let fields = exact_fields(arguments, 2, line, source, "COPY <src> <dst>")?;
        reject_runtime_variable(fields[0], "COPY source", line, source)?;
        reject_runtime_variable(fields[1], "COPY destination", line, source)?;
        let src = self.build_template(fields[0], line, source, false)?;
        validate_copy_source(&src, line, source)?;
        let block = self.current.clone().ok_or_else(|| {
            ParseError::new(
                line,
                source,
                "COPY must appear inside a BUILDER, SERVICE, or APP block under D47",
            )
        })?;
        let destination = match &block {
            CurrentBlock::Builder(_) => {
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
        if !self
            .destinations
            .get_mut(name)
            .expect("block destinations exist")
            .insert(destination.to_owned())
        {
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "{} destination {:?} is already populated",
                    self.names[name].kind, destination
                ),
            ));
        }
        let copy = Copy {
            src,
            dst: destination.to_owned(),
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

    fn run(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        let Some(CurrentBlock::Builder(name)) = self.current.clone() else {
            return Err(ParseError::new(
                line,
                source,
                "RUN is only legal inside a BUILDER block (D47 workshop/shipping-dock doctrine)",
            ));
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

    fn builder_command(
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

    fn push_builder_command(
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

    fn heredoc(
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
        };
        self.artifacts
            .get_mut(&artifact_name)
            .expect("artifact exists")
            .assembly
            .push(assembly);
        Ok(())
    }

    fn read_heredoc_body(
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
            format!("unterminated {directive} heredoc; expected {delimiter:?}"),
        ))
    }

    fn link(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
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
                "LINK argument order changed in D52; replace old `LINK <linkpath> <target>` with `LINK <target> <linkpath>`",
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
        self.claim_artifact_destination(destination, line, source)?;
        self.current_artifact_mut("LINK", line, source)?
            .assembly
            .push(Assembly::Link {
                dst: destination.to_owned(),
                target,
            });
        Ok(())
    }

    fn exec(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
        setup: bool,
    ) -> Result<(), ParseError> {
        let directive = if setup { "SETUP" } else { "EXEC" };
        let fields = argv_fields(arguments, line, source, directive)?;
        validate_artifact_command_path(&fields[0], directive, line, source)?;
        let artifact_name = self
            .current_artifact_name(directive, line, source)?
            .to_owned();
        let kind = self.artifacts[&artifact_name].kind;
        if !kind.is_runnable() {
            return Err(self.item_seam_parse_error(directive, line, source));
        }
        if kind == ArtifactKind::App && setup {
            return Err(ParseError::new(
                line,
                source,
                "SETUP is not legal inside APP blocks under D47; put preparation in the APP executable",
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
        if setup {
            if service.setup.is_some() {
                return Err(ParseError::new(
                    line,
                    source,
                    "SETUP is already declared for this service",
                ));
            }
            service.setup = Some(templates);
            service.setup_line = Some(line);
            self.metadata
                .get_mut(&artifact_name)
                .expect("artifact metadata exists")
                .setup = Some((line, source.to_owned()));
        } else {
            if !service.exec.is_empty() {
                return Err(ParseError::new(
                    line,
                    source,
                    "EXEC is already declared for this artifact",
                ));
            }
            service.exec = templates;
            service.exec_line = line;
            self.metadata
                .get_mut(&artifact_name)
                .expect("artifact metadata exists")
                .exec = Some((line, source.to_owned()));
        }
        Ok(())
    }

    fn env(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        if matches!(self.current, Some(CurrentBlock::Builder(_))) {
            let fields = exact_fields(arguments, 3, line, source, "ENV <name> = <plain-value>")?;
            validate_env_name(fields[0], line, source)?;
            if fields[1] != "=" {
                return Err(ParseError::new(
                    line,
                    source,
                    "builder ENV name must be followed by '='",
                ));
            }
            if fields[2].contains("${") {
                return Err(ParseError::new(
                    line,
                    source,
                    "builder ENV values are plain text and do not support build-time interpolation",
                ));
            }
            let name = self.current_builder_name("ENV", line, source)?.to_owned();
            self.builders
                .get_mut(&name)
                .expect("current builder exists")
                .steps
                .push(BuildStep::Env {
                    name: fields[0].to_owned(),
                    value: fields[2].to_owned(),
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
        let fields = at_least_one_field(arguments, line, source, "ENV")?;
        validate_env_name(fields[0], line, source)?;
        let mut index = 1;
        let default = if fields.get(index) == Some(&"=") {
            index += 1;
            let value = fields.get(index).ok_or_else(|| {
                ParseError::new(
                    line,
                    source,
                    "ENV '=' must be followed by one default value",
                )
            })?;
            index += 1;
            reject_runtime_variable(value, "ENV default", line, source)?;
            Some(self.build_template(value, line, source, false)?)
        } else {
            None
        };
        let mut required = false;
        let mut secret = false;
        for flag in &fields[index..] {
            match *flag {
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
        let artifact_name = self.current_artifact_name("ENV", line, source)?.to_owned();
        let service = &mut self
            .artifacts
            .get_mut(&artifact_name)
            .expect("artifact exists")
            .service;
        if service.env.contains_key(fields[0]) {
            return Err(ParseError::new(
                line,
                source,
                format!("ENV {:?} is already declared", fields[0]),
            ));
        }
        service.env.insert(
            fields[0].to_owned(),
            Env {
                default,
                required,
                secret,
            },
        );
        Ok(())
    }

    fn port(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        self.require_artifact_kind("PORT", line, source, &[ArtifactKind::Service])?;
        let fields = exact_fields(arguments, 3, line, source, "PORT <name> = <$VAR|value>")?;
        validate_name("port", fields[0], line, source)?;
        if fields[1] != "=" {
            return Err(ParseError::new(
                line,
                source,
                "PORT name must be followed by '='",
            ));
        }
        let port = if let Some(variable) = fields[2].strip_prefix('$') {
            validate_env_name(variable, line, source)?;
            Port::Env(variable.to_owned())
        } else {
            let value = fields[2].parse::<u16>().map_err(|_| {
                ParseError::new(line, source, "PORT value must be between 1 and 65535")
            })?;
            if value == 0 {
                return Err(ParseError::new(
                    line,
                    source,
                    "PORT value must be between 1 and 65535",
                ));
            }
            Port::Value(value)
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

    fn listener(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
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

    fn directory(
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
        reject_build_interpolation(fields[0], "directory path", line, source)?;
        reject_runtime_variable(fields[0], "directory path", line, source)?;
        let (root, role) = match directive {
            "STATEDIR" => ("/var/lib", "state"),
            "CACHEDIR" => ("/var/cache", "cache"),
            "LOGSDIR" => ("/var/log", "logs"),
            "CONFIGDIR" => ("/etc", "config"),
            "RUNDIR" => ("/run", "run"),
            _ => unreachable!(),
        };
        validate_role_path(fields[0], root, role, line, source)?;
        let service = self.current_service_mut(directive, line, source)?;
        let paths = match directive {
            "STATEDIR" => &mut service.dirs.state,
            "CACHEDIR" => &mut service.dirs.cache,
            "LOGSDIR" => &mut service.dirs.logs,
            "CONFIGDIR" => &mut service.dirs.config,
            "RUNDIR" => &mut service.dirs.run,
            _ => unreachable!(),
        };
        if !paths.insert(fields[0].to_owned()) {
            return Err(ParseError::new(
                line,
                source,
                format!("{directive} path {:?} is duplicated", fields[0]),
            ));
        }
        Ok(())
    }

    fn grant(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        self.require_artifact_kind(
            "GRANT",
            line,
            source,
            &[ArtifactKind::Service, ArtifactKind::App],
        )?;
        let fields = exact_fields(arguments, 1, line, source, "GRANT <jit|egress>")?;
        if !matches!(fields[0], "jit" | "egress") {
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "unknown GRANT capability {:?}; supported capabilities: jit, egress",
                    fields[0]
                ),
            ));
        }
        let service = self.current_service_mut("GRANT", line, source)?;
        if !service.grants.insert(fields[0].to_owned()) {
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "GRANT {:?} is already declared for this artifact",
                    fields[0]
                ),
            ));
        }
        Ok(())
    }

    fn current_service_mut(
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

    fn current_artifact_mut(
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

    fn current_artifact_name(
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

    fn current_builder_name(
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

    fn require_artifact_kind(
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
        Err(ParseError::new(
            line,
            source,
            format!(
                "{directive} is not legal inside {} blocks under D47",
                kind.keyword()
            ),
        ))
    }

    fn item_seam_error(&self, directive: &str, line: usize, source: &str) -> Option<ParseError> {
        const RUNTIME_DIRECTIVES: &[&str] = &[
            "EXEC",
            "SETUP",
            "ENV",
            "PORT",
            "LISTENER",
            "STATEDIR",
            "CACHEDIR",
            "LOGSDIR",
            "CONFIGDIR",
            "RUNDIR",
            "STATE",
            "LOGS",
            "CONFIG",
            "GRANT",
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

    fn item_seam_parse_error(&self, directive: &str, line: usize, source: &str) -> ParseError {
        ParseError::new(
            line,
            source,
            format!(
                "{directive} crosses the ITEM seam (D68): items are build products; SERVICE/APP declare runnable contracts"
            ),
        )
    }

    fn claim_artifact_destination(
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

    fn build_template(
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

    fn declare_name(
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
                    "name {name:?} is already bound by a {} on line {}; block and binder names share one namespace",
                    first.kind, first.line
                ),
            ));
        }
        self.names
            .insert(name.to_owned(), DeclaredName { kind, line });
        Ok(())
    }
}

fn heredoc_delimiter<'a>(
    arguments: &'a str,
    directive: &str,
    line: usize,
    source: &str,
) -> Result<Option<&'a str>, ParseError> {
    if !arguments.starts_with("<<") {
        return Ok(None);
    }
    let fields = exact_fields(arguments, 1, line, source, &format!("{directive} <<EOF"))?;
    let delimiter = fields[0]
        .strip_prefix("<<")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ParseError::new(
                line,
                source,
                format!("{directive} heredoc must use << followed by a delimiter"),
            )
        })?;
    Ok(Some(delimiter))
}

fn exact_fields<'a>(
    arguments: &'a str,
    count: usize,
    line: usize,
    source: &str,
    usage: &str,
) -> Result<Vec<&'a str>, ParseError> {
    let fields = arguments.split_whitespace().collect::<Vec<_>>();
    if fields.len() != count {
        return Err(ParseError::new(line, source, format!("expected {usage}")));
    }
    Ok(fields)
}

fn at_least_one_field<'a>(
    arguments: &'a str,
    line: usize,
    source: &str,
    directive: &str,
) -> Result<Vec<&'a str>, ParseError> {
    let fields = arguments.split_whitespace().collect::<Vec<_>>();
    if fields.is_empty() {
        return Err(ParseError::new(
            line,
            source,
            format!("{directive} requires at least one argument"),
        ));
    }
    Ok(fields)
}

fn argv_fields(
    arguments: &str,
    line: usize,
    source: &str,
    directive: &str,
) -> Result<Vec<String>, ParseError> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quote = None;
    let mut started = false;
    for character in arguments.chars() {
        match (quote, character) {
            (Some(delimiter), character) if character == delimiter => quote = None,
            (Some(_), character) => field.push(character),
            (None, '\'' | '"') => {
                quote = Some(character);
                started = true;
            }
            (None, character) if character.is_whitespace() => {
                if started {
                    fields.push(std::mem::take(&mut field));
                    started = false;
                }
            }
            (None, character) => {
                field.push(character);
                started = true;
            }
        }
    }
    if quote.is_some() {
        return Err(ParseError::new(
            line,
            source,
            format!("unterminated quote in {directive} arguments"),
        ));
    }
    if started {
        fields.push(field);
    }
    if fields.is_empty() {
        return Err(ParseError::new(
            line,
            source,
            format!("{directive} requires at least one argument"),
        ));
    }
    Ok(fields)
}

fn build_template(
    input: &str,
    line: usize,
    source: &str,
    heredoc: bool,
    inputs: &BTreeMap<String, Input>,
    names: &BTreeMap<String, DeclaredName>,
) -> Result<Template, ParseError> {
    let mut parts = Vec::new();
    let mut literal = String::new();
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if heredoc && bytes[index..].starts_with(b"$${") {
            let Some(close_offset) = input[index + 3..].find('}') else {
                return Err(ParseError::new(
                    line,
                    source,
                    "unterminated $${…} escape in heredoc",
                ));
            };
            let close = index + 3 + close_offset;
            literal.push_str("${");
            literal.push_str(&input[index + 3..close]);
            literal.push('}');
            index = close + 1;
            continue;
        }
        if bytes[index..].starts_with(b"${") {
            let Some(close_offset) = input[index + 2..].find('}') else {
                return Err(ParseError::new(
                    line,
                    source,
                    "unterminated ${…} build-time interpolation",
                ));
            };
            let close = index + 2 + close_offset;
            let reference = &input[index + 2..close];
            if let Some((namespace, attrpath)) = reference.split_once('.') {
                let Some(input) = inputs.get(namespace) else {
                    if names.contains_key(namespace) {
                        return Err(ParseError::new(
                            line,
                            source,
                            format!(
                                "binder {namespace:?} is a source tree; select paths as ${{{namespace}}}/<path>, not with attribute syntax"
                            ),
                        ));
                    }
                    let declared = inputs
                        .iter()
                        .filter(|(_, input)| input.kind == InputKind::PackageUniverse)
                        .map(|(name, _)| name.clone())
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(ParseError::new(
                        line,
                        source,
                        format!(
                            "unknown package namespace {namespace:?}; declared package namespaces: {declared}"
                        ),
                    ));
                };
                if input.kind == InputKind::Artifact {
                    return Err(ParseError::new(
                        line,
                        source,
                        format!(
                            "cix-item binder {namespace:?} is a tree; ${{{namespace}.…}} would make it a package namespace, which D65(c) forbids; use ${{{namespace}}}/<path>"
                        ),
                    ));
                }
                if input.kind != InputKind::PackageUniverse {
                    return Err(ParseError::new(
                        line,
                        source,
                        format!(
                            "FROM source binder {namespace:?} is a tree; use ${{{namespace}}}/<path>"
                        ),
                    ));
                }
                if !valid_attrpath(attrpath) {
                    return Err(ParseError::new(
                        line,
                        source,
                        "package interpolation must name a dot-separated attribute path after its namespace",
                    ));
                }
                if !literal.is_empty() {
                    parts.push(TemplatePart::Literal(std::mem::take(&mut literal)));
                }
                parts.push(TemplatePart::Package {
                    namespace: namespace.to_owned(),
                    attrpath: attrpath.to_owned(),
                    line,
                });
                index = close + 1;
                continue;
            }
            if let Some(input) = inputs.get(reference) {
                if input.kind == InputKind::PackageUniverse {
                    return Err(ParseError::new(
                        line,
                        source,
                        format!(
                            "package universe {reference:?} needs an attribute path, for example ${{{reference}.hello}}"
                        ),
                    ));
                }
            } else if let Some(declaration) = names.get(reference) {
                if declaration.kind == "artifact block" {
                    return Err(ParseError::new(
                        line,
                        source,
                        format!(
                            "artifact block {reference:?} is not a source binder; COPY from a FROM, FETCH, or BUILDER binder"
                        ),
                    ));
                }
            } else {
                if reference == "build" {
                    return Err(ParseError::new(
                        line,
                        source,
                        "no binder named `build`; name your builder: `BUILDER build`",
                    ));
                }
                if valid_attrpath(reference) {
                    if let Some(namespace) = inputs
                        .iter()
                        .find(|(_, input)| input.kind == InputKind::PackageUniverse)
                        .map(|(name, _)| name)
                    {
                        return Err(ParseError::new(
                            line,
                            source,
                            format!(
                                "no binder named {reference:?}; for a package, use ${{{namespace}.{reference}}}; binder references are backward-only"
                            ),
                        ));
                    }
                }
                return Err(ParseError::new(
                    line,
                    source,
                    format!("no binder named {reference:?}; binder references are backward-only"),
                ));
            }
            if !literal.is_empty() {
                parts.push(TemplatePart::Literal(std::mem::take(&mut literal)));
            }
            parts.push(TemplatePart::Binder {
                name: reference.to_owned(),
                line,
            });
            index = close + 1;
            continue;
        }
        let character = input[index..].chars().next().expect("index in bounds");
        literal.push(character);
        index += character.len_utf8();
    }
    if !literal.is_empty() || parts.is_empty() {
        parts.push(TemplatePart::Literal(literal));
    }
    Ok(Template { parts })
}

fn append_template(target: &mut Template, source: Template) {
    for part in source.parts {
        match part {
            TemplatePart::Literal(value) => push_literal(target, &value),
            TemplatePart::Package {
                namespace,
                attrpath,
                line,
            } => target.parts.push(TemplatePart::Package {
                namespace,
                attrpath,
                line,
            }),
            TemplatePart::Binder { name, line } => {
                target.parts.push(TemplatePart::Binder { name, line })
            }
        }
    }
}

fn push_literal(template: &mut Template, value: &str) {
    if let Some(TemplatePart::Literal(existing)) = template.parts.last_mut() {
        existing.push_str(value);
    } else {
        template.parts.push(TemplatePart::Literal(value.to_owned()));
    }
}

fn reject_build_interpolation(
    input: &str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    if input.contains("${") {
        return Err(ParseError::new(
            line,
            source,
            format!("{label} does not support build-time interpolation"),
        ));
    }
    Ok(())
}

fn reject_runtime_variable(
    input: &str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'$' {
            if bytes.get(index + 1) == Some(&b'{') {
                index += 2;
                continue;
            }
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "runtime $VAR interpolation is only allowed in EXEC and SETUP, not {label}"
                ),
            ));
        }
        index += 1;
    }
    Ok(())
}

fn validate_service_references(
    service: &Service,
    metadata: &ServiceMetadata,
) -> Result<(), ParseError> {
    let commands = [
        (&service.exec[..], metadata.exec.as_ref()),
        (
            service.setup.as_deref().unwrap_or_default(),
            metadata.setup.as_ref(),
        ),
    ];
    for (arguments, location) in commands {
        let Some((line, source)) = location else {
            continue;
        };
        for argument in arguments {
            for part in &argument.parts {
                if let TemplatePart::Literal(value) = part {
                    for variable in runtime_variables(value, *line, source)? {
                        if !service.env.contains_key(&variable) {
                            return Err(ParseError::new(
                                *line,
                                source,
                                format!(
                                    "EXEC/SETUP references undeclared environment variable ${variable}"
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
    for (name, port) in &service.ports {
        let (line, source) = &metadata.ports[name];
        if let Port::Env(variable) = port {
            let Some(env) = service.env.get(variable) else {
                return Err(ParseError::new(
                    *line,
                    source,
                    format!("PORT references undeclared environment variable ${variable}"),
                ));
            };
            if let Some(default) = &env.default {
                let Some(default) = default.literal_value() else {
                    return Err(ParseError::new(
                        *line,
                        source,
                        format!(
                            "PORT environment variable ${variable} must have a numeric default"
                        ),
                    ));
                };
                let valid = default.parse::<u16>().is_ok_and(|port| port != 0);
                if !valid {
                    return Err(ParseError::new(
                        *line,
                        source,
                        format!("PORT environment variable ${variable} must have a default between 1 and 65535"),
                    ));
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn bare_command(arguments: &[Template]) -> Option<String> {
    let command = arguments.first()?.literal_value()?;
    (!command.is_empty() && !command.contains('/') && !command.contains('$')).then_some(command)
}

fn validate_import_template(
    template: &Template,
    inputs: &BTreeMap<String, Input>,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    if let [TemplatePart::Binder { name, .. }] = template.parts.as_slice() {
        if inputs
            .get(name)
            .is_some_and(|input| input.kind == InputKind::Artifact)
        {
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "IMPORT of cix-item binder {name:?} is deferred by D65(d); cix items are COPY/LINK source trees, not builder imports"
                ),
            ));
        }
    }
    if matches!(
        template.parts.as_slice(),
        [TemplatePart::Package { .. } | TemplatePart::Binder { .. }]
    ) {
        return Ok(());
    }
    Err(ParseError::new(
        line,
        source,
        "IMPORT requires whole package references such as ${pkgs.coreutils}, without a /bin suffix",
    ))
}

fn validate_copy_source(template: &Template, line: usize, source: &str) -> Result<(), ParseError> {
    match template.parts.as_slice() {
        [TemplatePart::Literal(path)] => {
            validate_copy_relative_path(path, "COPY source", line, source)
        }
        [TemplatePart::Package { .. } | TemplatePart::Binder { .. }] => Ok(()),
        [
            TemplatePart::Package { .. } | TemplatePart::Binder { .. },
            TemplatePart::Literal(path),
        ] if path == "/" => Ok(()),
        [
            TemplatePart::Package { .. } | TemplatePart::Binder { .. },
            TemplatePart::Literal(path),
        ] if path.starts_with('/') => {
            validate_copy_relative_path(&path[1..], "COPY source", line, source)
        }
        _ => Err(ParseError::new(
            line,
            source,
            "COPY source must be one bare relative path or one binder/package path such as ${src}/sub/path or ${pkgs.hello}/bin/hello",
        )),
    }
}

fn parse_fetch_expect<'a>(
    arguments: &'a str,
    line: usize,
    source: &str,
) -> Result<(Option<String>, &'a str), ParseError> {
    let Some(remainder) = arguments.strip_prefix("EXPECT") else {
        return Ok((None, arguments));
    };
    if remainder
        .chars()
        .next()
        .is_some_and(|character| !character.is_whitespace())
    {
        return Ok((None, arguments));
    }
    let remainder = remainder.trim_start();
    let (hash, command) = remainder.split_once(char::is_whitespace).ok_or_else(|| {
        ParseError::new(
            line,
            source,
            "FETCH EXPECT requires a hash and command: EXPECT <sri-hash> <command…>",
        )
    })?;
    if !hash.starts_with("sha256-") || hash.len() == "sha256-".len() {
        return Err(ParseError::new(
            line,
            source,
            format!("FETCH EXPECT hash must be an SRI sha256 hash, got {hash:?}"),
        ));
    }
    let command = command.trim();
    if command.is_empty() {
        return Err(ParseError::new(
            line,
            source,
            "FETCH EXPECT requires a command after the hash",
        ));
    }
    Ok((Some(hash.to_owned()), command))
}

fn runtime_variables(
    input: &str,
    line: usize,
    source: &str,
) -> Result<BTreeSet<String>, ParseError> {
    let bytes = input.as_bytes();
    let mut variables = BTreeSet::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        index += 1;
        if index >= bytes.len() || !is_env_start(bytes[index]) {
            return Err(ParseError::new(
                line,
                source,
                "runtime '$' must be followed by an environment variable name",
            ));
        }
        let start = index;
        index += 1;
        while index < bytes.len() && is_env_continue(bytes[index]) {
            index += 1;
        }
        variables.insert(input[start..index].to_owned());
    }
    Ok(variables)
}

fn valid_attrpath(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(valid_attr_component)
}

fn valid_attr_component(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|value| value.is_ascii_alphabetic() || value == b'_')
        && bytes.all(|value| value.is_ascii_alphanumeric() || matches!(value, b'_' | b'-' | b'\''))
}

fn validate_namespace(value: &str, line: usize, source: &str) -> Result<(), ParseError> {
    if !valid_attr_component(value) {
        return Err(ParseError::new(
            line,
            source,
            "FROM namespace must be a Nix-style identifier",
        ));
    }
    Ok(())
}

fn normalize_input(
    value: &str,
    line: usize,
    source: &str,
) -> Result<(String, InputKind), ParseError> {
    if value == "." || value.starts_with("./") {
        return Ok((value.to_owned(), InputKind::Source));
    }
    let github = value.strip_prefix("github:").is_some_and(|path| {
        let parts = path.split('/').collect::<Vec<_>>();
        (2..=3).contains(&parts.len()) && parts.iter().all(|part| !part.is_empty())
    });
    if github
        || value.starts_with("git+")
        || value.starts_with("path:")
        || value.starts_with("tarball+")
    {
        let kind = if value
            .strip_prefix("github:")
            .is_some_and(|path| path.starts_with("NixOS/nixpkgs/") || path == "NixOS/nixpkgs")
        {
            InputKind::PackageUniverse
        } else {
            InputKind::Source
        };
        return Ok((value.to_owned(), kind));
    }
    match cix_common::Ref::parse(value) {
        Ok(reference) => Ok((reference.display(), InputKind::Artifact)),
        Err(error) => Err(ParseError::new(
            line,
            source,
            format!(
                "FROM input must be a known flakeref (github:, git+, path:, tarball+, ., or ./…) or an index ref with an explicit :tag; {error}"
            ),
        )),
    }
}

fn take_removed_error(line: usize, source: &str, arguments: &str) -> ParseError {
    let mut fields = arguments.split_whitespace();
    let rewrite = match (fields.next(), fields.next(), fields.next()) {
        (Some(from), Some(to), None) => format!(
            "TAKE was removed by D47; inside the artifact block use COPY ${{build}}/{from} {to}, and name the producing block `BUILDER build`"
        ),
        _ => "TAKE was removed by D47; use COPY ${<builder>}/<path> <destination> inside the artifact block".to_owned(),
    };
    ParseError::new(line, source, rewrite)
}

fn pkg_removed_error(line: usize, source: &str, arguments: &str) -> ParseError {
    let rewrite = arguments
        .split_whitespace()
        .next()
        .filter(|attribute| valid_attrpath(attribute))
        .map_or_else(
            || "PKG was removed by D32; reference packages directly as ${pkgs.<attrpath>}"
                .to_owned(),
            |attribute| {
                format!(
                    "PKG was removed by D32; delete this line and replace ${{{attribute}}} with ${{pkgs.{attribute}}}"
                )
            },
        );
    ParseError::new(line, source, rewrite)
}

fn validate_name(kind: &str, value: &str, line: usize, source: &str) -> Result<(), ParseError> {
    let mut bytes = value.bytes();
    if !bytes
        .next()
        .is_some_and(|value| value.is_ascii_alphanumeric())
        || !bytes.all(|value| value.is_ascii_alphanumeric() || matches!(value, b'_' | b'.' | b'-'))
    {
        return Err(ParseError::new(
            line,
            source,
            format!(
                "{kind} name must start with an ASCII letter or digit and contain only letters, digits, '.', '-', or '_'"
            ),
        ));
    }
    Ok(())
}

fn validate_env_name(value: &str, line: usize, source: &str) -> Result<(), ParseError> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || !is_env_start(bytes[0])
        || !bytes[1..].iter().copied().all(is_env_continue)
    {
        return Err(ParseError::new(
            line,
            source,
            "environment variable name must match [A-Za-z_][A-Za-z0-9_]*",
        ));
    }
    Ok(())
}

fn is_env_start(value: u8) -> bool {
    value.is_ascii_alphabetic() || value == b'_'
}

fn is_env_continue(value: u8) -> bool {
    is_env_start(value) || value.is_ascii_digit()
}

fn validate_item_path(
    value: &str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    if Path::new(value).is_absolute() {
        validate_projected_path(value, label, line, source)
    } else {
        validate_relative_path(value, label, line, source)
    }
}

fn normalize_artifact_copy_destination<'a>(
    value: &'a str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<&'a str, ParseError> {
    if value == "/" {
        return Ok(".");
    }
    normalize_artifact_destination(value, label, line, source)
}

fn normalize_artifact_destination<'a>(
    value: &'a str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<&'a str, ParseError> {
    let Some(relative) = value.strip_prefix('/') else {
        return Err(ParseError::new(
            line,
            source,
            format!("{label} must be an absolute item-world path under D66; write /{value}"),
        ));
    };
    validate_projected_path(value, label, line, source)?;
    Ok(relative)
}

fn validate_artifact_command_path(
    value: &str,
    directive: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    if !value.contains("${") && value.contains('/') && !value.starts_with('/') {
        return Err(ParseError::new(
            line,
            source,
            format!("{directive} path {value:?} must be absolute in an artifact block under D66; write /{value}"),
        ));
    }
    Ok(())
}

fn validate_projected_path(
    value: &str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    let path = Path::new(value);
    if value == "/" {
        return Err(ParseError::new(
            line,
            source,
            format!("{label} is denied by the D22 v3 filesystem-projection rule"),
        ));
    }
    if value.ends_with('/')
        || value.contains("//")
        || value
            .split('/')
            .any(|component| matches!(component, "." | ".."))
    {
        return Err(ParseError::new(
            line,
            source,
            format!("{label} must be a normalized absolute path"),
        ));
    }
    if denied_projected_path(path) {
        return Err(ParseError::new(
            line,
            source,
            format!("{label} is denied by the D22 v3 filesystem-projection rule"),
        ));
    }
    Ok(())
}

fn denied_projected_path(path: &Path) -> bool {
    [
        "/nix",
        "/proc",
        "/sys",
        "/dev",
        "/run",
        "/var/lib",
        "/var/cache",
        "/var/log",
        "/etc/passwd",
        "/etc/group",
        "/etc/nsswitch.conf",
        "/etc",
        "/usr",
        "/bin",
    ]
    .iter()
    .any(|denied| path == Path::new(denied))
        || path.parent() == Some(Path::new("/"))
            && path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("lib"))
}

fn validate_relative_path(
    value: &str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ParseError::new(
            line,
            source,
            format!("{label} must be a clean relative path"),
        ));
    }
    Ok(())
}

fn validate_copy_relative_path(
    value: &str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    if value == "." {
        return Ok(());
    }
    validate_relative_path(value, label, line, source)
}

fn validate_role_path(
    value: &str,
    root: &str,
    role: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    let path = Path::new(value);
    let relative = path.strip_prefix(root).ok();
    let one_component = relative.is_some_and(|relative| {
        let mut components = relative.components();
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
    });
    if !one_component {
        return Err(ParseError::new(
            line,
            source,
            format!("{role} directory must be exactly one component under {root}"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod d47_tests {
    use super::*;

    #[test]
    fn parses_blocks_binders_and_both_artifact_kinds() {
        let parsed = parse(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
FETCH ingredient ${pkgs.coreutils}/bin/printf payload
BUILDER build
IMPORT ${pkgs.bash}
COPY Cargo.toml Cargo.toml
FETCH printf fetched > fetched
RUN cp fetched built
SERVICE web
COPY ${build}/built /bin/web
FILE /etc/app.conf <<E
source=${src}
E
	LINK ${pkgs.bash}/bin/bash /bin/sh
	ENV PATH = bin
EXEC web
SETUP /bin/web
ENV PORT = 8080 required
PORT http = $PORT
LISTENER admin
STATEDIR /var/lib/web
	CACHEDIR /var/cache/web
LOGSDIR /var/log/web
CONFIGDIR /etc/web
RUNDIR /run/web
GRANT jit
GRANT egress
APP migrate
COPY ${ingredient} /payload
EXEC /bin/true
ENV MODE = once
STATEDIR /var/lib/migrate
	CACHEDIR /var/cache/migrate
	GRANT egress
	"#,
        )
        .unwrap();
        assert_eq!(parsed.fetch_order, ["ingredient"]);
        assert_eq!(parsed.builder_order, ["build"]);
        assert_eq!(parsed.artifact_order, ["web", "migrate"]);
        assert_eq!(parsed.builders["build"].steps.len(), 3);
        assert_eq!(parsed.artifacts["web"].kind, ArtifactKind::Service);
        assert_eq!(parsed.artifacts["migrate"].kind, ArtifactKind::App);
    }

    #[test]
    fn fetch_expect_parses_in_both_forms_and_validates_the_hash() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFETCH ingredient EXPECT sha256-top ${pkgs.coreutils}/bin/printf top\nBUILDER build\nIMPORT ${pkgs.bash}\nFETCH EXPECT sha256-step printf step\nSERVICE app\nEXEC /bin/true\n",
        )
        .unwrap();
        assert_eq!(
            parsed.fetches["ingredient"].expected.as_deref(),
            Some("sha256-top")
        );
        let BuildStep::Fetch { expected, .. } = &parsed.builders["build"].steps[0] else {
            panic!("expected in-builder FETCH");
        };
        assert_eq!(expected.as_deref(), Some("sha256-step"));

        let error = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFETCH ingredient EXPECT not-sri printf payload\nSERVICE app\nEXEC /bin/true\n",
        )
        .unwrap_err();
        assert_eq!(error.line, 2);
        assert!(error.message.contains("SRI sha256"), "{error}");
    }

    #[test]
    fn import_accepts_whole_package_refs_and_path_has_migration_errors() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nIMPORT ${pkgs.bash}\nIMPORT ${pkgs.coreutils}\nRUN true\nSERVICE app\nEXEC /bin/true\n",
        )
        .unwrap();
        assert_eq!(parsed.builders["build"].imports.len(), 2);

        let suffix = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nIMPORT ${pkgs.bash}/bin\nSERVICE app\nEXEC /bin/true\n",
        )
        .unwrap_err();
        assert!(
            suffix.message.contains("whole package references"),
            "{suffix}"
        );

        let builder_path = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nPATH ${pkgs.bash}/bin\nSERVICE app\nEXEC /bin/true\n",
        )
        .unwrap_err();
        assert_eq!(builder_path.message, "PATH was replaced by IMPORT (D58)");
        let service_path =
            parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nPATH ${pkgs.bash}/bin\nEXEC /bin/true\n")
                .unwrap_err();
        assert_eq!(
            service_path.message,
            "PATH was removed (D58); declare ENV PATH = ... explicitly"
        );
    }

    #[test]
    fn bare_and_explicit_local_copy_sources_coexist() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM . AS src\nBUILDER build\nCOPY bare.txt bare\nCOPY ${src}/explicit.txt explicit\nSERVICE app\nCOPY bare.txt /bare\nCOPY ${src}/explicit.txt /explicit\nEXEC /bin/true\n",
        )
        .unwrap();
        let BuildStep::Copy(bare) = &parsed.builders["build"].steps[0] else {
            panic!("expected COPY");
        };
        assert_eq!(bare.src, Template::literal("bare.txt"));
        assert!(matches!(
            parsed.builders["build"].steps[1],
            BuildStep::Copy(Copy { .. })
        ));
        assert_eq!(parsed.artifacts["app"].copies.len(), 2);
    }

    #[test]
    fn bare_artifact_commands_need_no_explicit_path() {
        let parsed = parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nSETUP setup\nEXEC app\n").unwrap();
        assert_eq!(
            parsed.artifacts["app"].service.exec[0].literal_value(),
            Some("app".into())
        );
        assert_eq!(
            parsed.artifacts["app"].service.setup.as_ref().unwrap()[0].literal_value(),
            Some("setup".into())
        );
    }

    #[test]
    fn names_share_one_namespace_and_references_are_backward_only() {
        let duplicate =
            parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER pkgs\nRUN true\nSERVICE app\nEXEC /bin/true\n")
                .unwrap_err();
        assert_eq!(duplicate.line, 2);
        assert!(duplicate.message.contains("line 1"), "{duplicate}");
        assert!(
            duplicate.message.contains("share one namespace"),
            "{duplicate}"
        );

        let forward = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER final\nCOPY ${prior}/x x\nBUILDER prior\nCOPY x x\nSERVICE app\nEXEC /bin/true\n",
        )
        .unwrap_err();
        assert_eq!(forward.line, 3);
        assert!(forward.message.contains("backward-only"), "{forward}");

        let cycle = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nCOPY ${build}/x x\nSERVICE app\nEXEC /bin/true\n",
        )
        .unwrap_err();
        assert_eq!(cycle.line, 3);
        assert!(cycle.message.contains("cannot reference itself"), "{cycle}");
    }

    #[test]
    fn migration_errors_name_the_d47_rewrite() {
        for (input, line, message) in [
            (
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nRUN true\nSERVICE app\nEXEC /bin/true\n",
                2,
                "RUN is only legal inside a BUILDER block",
            ),
            (
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nCOPY ${build}/bin/app /bin/app\nEXEC /bin/app\n",
                3,
                "no binder named `build`; name your builder: `BUILDER build`",
            ),
            (
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nTAKE bin/app /bin/app\nEXEC /bin/app\n",
                3,
                "TAKE was removed by D47",
            ),
            (
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nPATH ${pkgs.bash}/bin\nSERVICE app\nEXEC bash\n",
                2,
                "PATH was replaced by IMPORT (D58)",
            ),
        ] {
            let error = parse(input).unwrap_err();
            assert_eq!(error.line, line, "{error}");
            assert!(error.message.contains(message), "{error}");
            assert!(error.to_string().contains(&format!("{:?}", error.source)));
        }
    }

    #[test]
    fn outbound_has_a_d48_migration_error_and_is_not_an_alias() {
        let error =
            parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nEXEC /bin/true\nOUTBOUND\n").unwrap_err();
        assert_eq!(error.line, 4);
        assert!(error.message.contains("GRANT egress"), "{error}");
        assert!(error.message.contains("D60"), "{error}");
    }

    #[test]
    fn script_has_the_d55_migration_error_and_is_not_an_alias() {
        let source = "SCRIPT bin/start <<EOF";
        let error = parse(&format!(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nEXEC /bin/true\n{source}\ntrue\nEOF\n"
        ))
        .unwrap_err();
        assert_eq!(error.line, 4);
        assert_eq!(error.source, source);
        assert_eq!(
            error.message,
            "SCRIPT was dropped (D55); COPY a real script and EXEC ${pkgs.bash}/bin/sh <path>, or use FILE if the content needs store-path interpolation"
        );
    }

    #[test]
    fn comments_continuations_and_run_heredocs_preserve_shell_text() {
        let parsed = parse(
            r#"# The package universe is intentionally split across physical lines.
FROM github:NixOS/nixpkgs/nixos-unstable \
    AS pkgs

BUILDER build
IMPORT ${pkgs.bash} \
    ${pkgs.coreutils}
# This comment is ignored by the Cixfile parser.
RUN printf '%s\n' \
    '# inline shell comment text is data' > continued
RUN <<SCRIPT
# This comment belongs to the builder shell.
printf '%s\n' ${pkgs.hello} > result
SCRIPT

SERVICE app
EXEC /bin/true \
    # this is an argument, not a Cixfile comment
"#,
        )
        .unwrap();
        assert_eq!(parsed.builders["build"].imports.len(), 2);
        let BuildStep::Run {
            command,
            line,
            source,
        } = &parsed.builders["build"].steps[0]
        else {
            panic!("expected continued RUN");
        };
        assert_eq!(*line, 9);
        assert!(source.starts_with("RUN printf"));
        assert_eq!(
            command.literal_value().as_deref(),
            Some("printf '%s\\n' '# inline shell comment text is data' > continued")
        );
        let BuildStep::Run { command, line, .. } = &parsed.builders["build"].steps[1] else {
            panic!("expected heredoc RUN");
        };
        assert_eq!(*line, 11);
        assert!(matches!(
            command.parts.as_slice(),
            [
                TemplatePart::Literal(first),
                TemplatePart::Package { line: 13, .. },
                TemplatePart::Literal(last),
            ] if first.starts_with("# This comment belongs") && last.ends_with(" > result\n")
        ));
        let exec = &parsed.artifacts["app"].service.exec;
        assert_eq!(exec[1].literal_value().as_deref(), Some("#"));
        assert_eq!(
            exec.last().and_then(Template::literal_value).as_deref(),
            Some("comment")
        );
    }

    #[test]
    fn run_heredoc_errors_use_physical_body_lines() {
        let error = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nRUN <<SCRIPT\ntrue\nprintf ${missing}\nSCRIPT\nSERVICE app\nEXEC /bin/true\n",
        )
        .unwrap_err();
        assert_eq!(error.line, 5, "{error}");
        assert_eq!(error.source, "printf ${missing}");

        let dangling = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nEXEC /bin/true \\\n",
        )
        .unwrap_err();
        assert_eq!(dangling.line, 3, "{dangling}");
        assert!(dangling.message.contains("continuation"), "{dangling}");

        let continued = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nIMPORT ${pkgs.bash} \\\n    ${missing.tool}\nSERVICE app\nEXEC /bin/true\n",
        )
        .unwrap_err();
        assert_eq!(continued.line, 4, "{continued}");
        assert_eq!(continued.source.trim(), "${missing.tool}");
    }

    #[test]
    fn cixfile_comments_are_full_line_only() {
        let parsed = parse(
            "  # ignored before the first declaration \\\nFROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\n# ignored in a block\nEXEC /bin/echo #kept\n",
        )
        .unwrap();
        let exec = &parsed.artifacts["app"].service.exec;
        assert_eq!(exec.len(), 2);
        assert_eq!(exec[1].literal_value().as_deref(), Some("#kept"));
    }

    #[test]
    fn cachedir_and_link_use_the_d52_spellings() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nLINK ${pkgs.hello}/bin/hello /bin/hello\nEXEC /bin/hello\nCACHEDIR /var/cache/app\n",
        )
        .unwrap();
        assert!(parsed.artifacts["app"]
            .service
            .dirs
            .cache
            .contains("/var/cache/app"));
        let Assembly::Link { dst, target } = &parsed.artifacts["app"].assembly[0] else {
            panic!("expected LINK");
        };
        assert_eq!(dst, "bin/hello");
        assert!(matches!(
            target.parts.first(),
            Some(TemplatePart::Package { attrpath, .. }) if attrpath == "hello"
        ));

        let cache =
            parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nEXEC /bin/true\nCACHE /var/cache/app\n")
                .unwrap_err();
        assert_eq!(cache.line, 4);
        assert_eq!(
            cache.message,
            "CACHE was removed (D57): workspaces persist by default and nothing is keyed unless read; delete the line"
        );

        let link = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nLINK bin/hello ${pkgs.hello}/bin/hello\nEXEC /bin/hello\n",
        )
        .unwrap_err();
        assert_eq!(link.line, 3);
        assert!(link.message.contains("argument order changed in D52"));
        assert!(link.message.contains("LINK <target> <linkpath>"));
    }

    #[test]
    fn builder_env_is_ordered_plain_text_and_exec_argv_is_quote_aware() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nIMPORT ${pkgs.bash}\nENV COREPACK_HOME = $PWD/.corepack\nRUN printf '%s\\n' ok\nSERVICE web\nEXEC ${pkgs.nginx}/bin/nginx -g 'daemon off;'\n",
        )
        .unwrap();
        assert!(matches!(
            &parsed.builders["build"].steps[0],
            BuildStep::Env { name, value, .. } if name == "COREPACK_HOME" && value == "$PWD/.corepack"
        ));
        assert_eq!(
            parsed.artifacts["web"].service.exec[2]
                .literal_value()
                .as_deref(),
            Some("daemon off;")
        );

        let unterminated = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nEXEC ${pkgs.nginx}/bin/nginx -g 'daemon off;\n",
        )
        .unwrap_err();
        assert_eq!(unterminated.line, 3);
        assert!(
            unterminated.message.contains("unterminated quote"),
            "{unterminated}"
        );
    }

    #[test]
    fn role_directory_directives_and_grant_are_hard_migrations() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nEXEC /bin/true\nSTATEDIR /var/lib/web\nCACHEDIR /var/cache/web\nLOGSDIR /var/log/web\nCONFIGDIR /etc/web\nRUNDIR /run/web\nGRANT jit\nGRANT egress\n",
        )
        .unwrap();
        let dirs = &parsed.artifacts["web"].service.dirs;
        assert!(dirs.state.contains("/var/lib/web"));
        assert!(dirs.cache.contains("/var/cache/web"));
        assert!(dirs.logs.contains("/var/log/web"));
        assert!(dirs.config.contains("/etc/web"));
        assert!(dirs.run.contains("/run/web"));
        assert_eq!(
            parsed.artifacts["web"].service.grants,
            BTreeSet::from(["egress".into(), "jit".into()])
        );
        for (directive, replacement, decision) in [
            ("STATE /var/lib/web", "STATEDIR", "D52"),
            ("LOGS /var/log/web", "LOGSDIR", "D52"),
            ("CONFIG /etc/web", "CONFIGDIR", "D52"),
            ("JIT", "GRANT jit", "D60"),
            ("EGRESS", "GRANT egress", "D60"),
        ] {
            let error = parse(&format!(
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nEXEC /bin/true\n{directive}\n"
            ))
            .unwrap_err();
            assert_eq!(error.line, 4);
            assert!(error.message.contains(replacement), "{error}");
            assert!(error.message.contains(decision), "{error}");
        }
        let unknown =
            parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nEXEC /bin/true\nGRANT all\n").unwrap_err();
        assert!(unknown.message.contains("jit, egress"), "{unknown}");
    }

    #[test]
    fn app_rejects_service_only_surface_at_the_directive_line() {
        for (directive, message) in [
            ("PORT http = 8080", "PORT is not legal inside APP"),
            ("LISTENER http", "LISTENER is not legal inside APP"),
            ("JIT", "JIT was replaced by GRANT jit"),
            ("SETUP /bin/true", "SETUP is not legal inside APP"),
            ("LOGSDIR /var/log/job", "LOGSDIR is not legal inside APP"),
            ("CONFIGDIR /etc/job", "CONFIGDIR is not legal inside APP"),
            ("RUNDIR /run/job", "RUNDIR is not legal inside APP"),
            (
                "PATH bin",
                "PATH was removed (D58); declare ENV PATH = ... explicitly",
            ),
        ] {
            let input = format!("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nAPP job\nEXEC /bin/true\n{directive}\n");
            let error = parse(&input).unwrap_err();
            assert_eq!(error.line, 4, "{directive}: {error}");
            assert!(error.message.contains(message), "{directive}: {error}");
        }
    }

    #[test]
    fn item_is_pure_assembly_and_runtime_directives_name_the_d68_seam() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nITEM data\nCOPY payload /payload\nFILE /share/message <<EOF\nhello\nEOF\nLINK ${pkgs.hello}/bin/hello /bin/hello\n",
        )
        .unwrap();
        assert_eq!(parsed.artifacts["data"].kind, ArtifactKind::Item);
        assert_eq!(parsed.artifacts["data"].copies.len(), 1);
        assert_eq!(parsed.artifacts["data"].assembly.len(), 2);

        for directive in [
            "EXEC /bin/hello",
            "SETUP /bin/hello",
            "ENV PATH = bin",
            "PORT http = 8080",
            "LISTENER http",
            "STATEDIR /var/lib/data",
            "CACHEDIR /var/cache/data",
            "LOGSDIR /var/log/data",
            "CONFIGDIR /etc/data",
            "RUNDIR /run/data",
            "GRANT egress",
            "HEALTH /bin/hello",
        ] {
            let input = format!(
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nITEM data\n{directive}\n"
            );
            let error = parse(&input).unwrap_err();
            assert_eq!(error.line, 3, "{directive}: {error}");
            assert!(
                error.message.contains("ITEM seam (D68)"),
                "{directive}: {error}"
            );
            assert!(
                error
                    .message
                    .contains("items are build products; SERVICE/APP declare runnable contracts"),
                "{directive}: {error}"
            );
        }
    }

    #[test]
    fn source_and_package_interpolation_are_distinct() {
        let tree_attr = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM github:owner/repo AS src\nSERVICE app\nCOPY ${src.subdir} subdir\nEXEC /bin/true\n",
        )
        .unwrap_err();
        assert!(tree_attr.message.contains("${src}/<path>"), "{tree_attr}");

        let universe_tree =
            parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nCOPY ${pkgs} pkgs\nEXEC /bin/true\n")
                .unwrap_err();
        assert!(
            universe_tree.message.contains("needs an attribute path"),
            "{universe_tree}"
        );
    }

    #[test]
    fn builder_destinations_are_relative_and_artifact_destinations_are_absolute() {
        parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nCOPY . .\nSERVICE app\nCOPY ${build} /\nEXEC /bin/true\n",
        )
        .unwrap();
        for (destination, spelling) in [("relative", "/relative"), (".", "/")] {
            let input = format!(
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nCOPY payload {destination}\nEXEC /bin/true\n"
            );
            let error = parse(&input).unwrap_err();
            assert_eq!(error.line, 3);
            assert!(
                error.message.contains("absolute item-world path"),
                "{destination}: {error}"
            );
            assert!(error.message.contains(spelling), "{destination}: {error}");
        }
        let builder = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nCOPY payload /bad\nSERVICE app\nEXEC /bin/true\n",
        )
        .unwrap_err();
        assert!(builder.message.contains("clean relative path"), "{builder}");

        for directive in [
            "FILE etc/app.conf <<EOF\nvalue\nEOF",
            "LINK /nix/store/target bin/tool",
            "EXEC bin/tool",
            "SETUP bin/tool\nEXEC /bin/true",
        ] {
            let input = format!("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\n{directive}\nEXEC /bin/true\n");
            let error = parse(&input).unwrap_err();
            assert!(error.message.contains("D66"), "{directive}: {error}");
            assert!(error.message.contains("/bin") || error.message.contains("/etc"));
        }
    }

    #[test]
    fn from_local_is_optional_but_a_package_universe_is_required() {
        parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nCOPY payload /payload\nEXEC /bin/true\n")
            .unwrap();
        let error =
            parse("FROM . AS src\nSERVICE data\nCOPY ${src}/payload /payload\nEXEC /bin/true\n")
                .unwrap_err();
        assert!(error.message.contains("package universe"), "{error}");
    }

    #[test]
    fn from_cix_item_is_an_artifact_tree_with_d65_errors() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM family/web:v3 AS webvault\nSERVICE app\nCOPY ${webvault}/payload /payload\nEXEC /bin/true\n",
        )
        .unwrap();
        assert_eq!(parsed.inputs["webvault"].kind, InputKind::Artifact);
        assert_eq!(parsed.inputs["webvault"].url, "family/web:v3");

        let untagged = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM family/web AS webvault\nSERVICE app\nEXEC /bin/true\n",
        )
        .unwrap_err();
        assert!(untagged.message.contains("flakeref"), "{untagged}");
        assert!(untagged.message.contains(":latest"), "{untagged}");

        let attr_use = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM family/web:v3 AS webvault\nSERVICE app\nCOPY ${webvault.payload} /payload\nEXEC /bin/true\n",
        )
        .unwrap_err();
        assert!(attr_use.message.contains("D65(c)"), "{attr_use}");

        let import = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM family/web:v3 AS webvault\nBUILDER build\nIMPORT ${webvault}\nSERVICE app\nEXEC /bin/true\n",
        )
        .unwrap_err();
        assert!(import.message.contains("D65(d)"), "{import}");
    }
}
