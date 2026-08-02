//! The parser state machine owns lexical traversal and final assembly.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::*;

use super::migrations::{pkg_removed_error, take_removed_error};
use super::{diagnostics, validate::*};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub source: String,
    pub message: String,
}

impl ParseError {
    pub(super) fn new(line: usize, source: &str, message: impl Into<String>) -> Self {
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

pub(super) struct Parser<'a> {
    pub(super) lines: Vec<&'a str>,
    pub(super) index: usize,
    pub(super) inputs: BTreeMap<String, Input>,
    pub(super) fetches: BTreeMap<String, Fetch>,
    pub(super) fetch_order: Vec<String>,
    pub(super) builders: BTreeMap<String, Builder>,
    pub(super) builder_order: Vec<String>,
    pub(super) artifacts: BTreeMap<String, Artifact>,
    pub(super) artifact_order: Vec<String>,
    pub(super) names: BTreeMap<String, DeclaredName>,
    pub(super) destinations: BTreeMap<String, BTreeSet<String>>,
    pub(super) metadata: BTreeMap<String, ServiceMetadata>,
    pub(super) current: Option<CurrentBlock>,
}

#[derive(Clone)]
pub(super) struct DeclaredName {
    pub(super) kind: &'static str,
    pub(super) line: usize,
}

#[derive(Clone)]
pub(super) enum CurrentBlock {
    Builder(String),
    Artifact(String),
}

#[derive(Default)]
pub(super) struct ServiceMetadata {
    pub(super) start: Option<(usize, String)>,
    pub(super) start_pre: Option<(usize, String)>,
    pub(super) ports: BTreeMap<String, (usize, String)>,
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
                       "directive line continuation has no next line; remove the trailing backslash or add the continuation",
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
                       "directive line continuation has no next line; remove the trailing backslash or add the continuation",
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
                "ITEM" => {
                    self.begin_artifact(ArtifactKind::Item, line_number, source, arguments)?
                }
                "COPY" => self.copy(line_number, source, arguments)?,
                "RUN" => self.run(line_number, source, arguments)?,
                "IMPORT" => self.import(line_number, source, arguments)?,
                "PATH" => {
                    let message = match &self.current {
                       Some(CurrentBlock::Builder(_)) => "PATH was removed; use IMPORT ${pkgs.<package>} inside a BUILDER; see docs/cixfile.md#builders",
                       Some(CurrentBlock::Artifact(_)) => {
                           "PATH was removed; use ENV PATH = <value>; see docs/cixfile.md#runtime-path"
                       }
                       None => "PATH was removed; use IMPORT ${pkgs.<package>} inside a BUILDER; see docs/cixfile.md#builders",
                   };
                    return Err(ParseError::new(line_number, source, message));
                }
                "CACHE" => {
                    return Err(ParseError::new(
                       line_number,
                       source,
                       "CACHE was removed; delete this line because builder workspaces persist automatically; see docs/cixfile.md#builders",
                   ));
                }
                "FILE" => self.heredoc(directive, line_number, source, arguments)?,
                "SCRIPT" => {
                    return Err(ParseError::new(
                       line_number,
                       source,
                       "SCRIPT was removed; COPY a script and invoke it with START ${pkgs.bash}/bin/sh /path; see docs/cixfile.md#copy",
                   ));
                }
                "LINK" => self.link(line_number, source, arguments)?,
                "START" => self.start(line_number, source, arguments, false)?,
                "START_PRE" => self.start(line_number, source, arguments, true)?,
                "ENV" => self.env(line_number, source, arguments)?,
                "PORT" => self.port(line_number, source, arguments)?,
                "LISTENER" => self.listener(line_number, source, arguments)?,
                "SHM" => self.shm(line_number, source, arguments)?,
                "STATEDIR" | "CACHEDIR" | "LOGDIR" | "CONFIGDIR" | "RUNDIR" | "DIR" => {
                    self.directory(directive, line_number, source, arguments)?
                }
                "DATADIR" => {
                    return Err(ParseError::new(
                        line_number,
                        source,
                        "DIR declares operator-supplied data; materialization arrives with compose (docs/cixfile.md#role-dirs); for a cix-managed dir pick a role: STATEDIR/CACHEDIR/LOGDIR/RUNDIR",
                    ));
                }
                "EXEC" | "SETUP" | "STATE" | "LOGS" | "CONFIG" | "JIT" | "EGRESS" | "OUTBOUND"
                | "GRANT" => {
                    return Err(ParseError::new(
                        line_number,
                        source,
                        diagnostics::migration(directive).expect("migration is declared"),
                    ));
                }
                "CLAIM" => self.claim(line_number, source, arguments)?,
                "TAKE" => return Err(take_removed_error(line_number, source, arguments)),
                "PKG" => return Err(pkg_removed_error(line_number, source, arguments)),
                _ => {
                    return Err(ParseError::new(
                        line_number,
                        source,
                        diagnostics::unknown_directive(directive),
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
               "no package universe is declared; Docker images are not inherited here; add FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs; see docs/migrate.md#docker-vocabulary",
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
            if artifact.kind.is_runnable() && artifact.service.start.is_empty() {
                return Err(ParseError::new(
                    artifact.line,
                    self.lines
                        .get(artifact.line - 1)
                        .copied()
                        .unwrap_or_default(),
                    format!(
                        "{} {name:?} has no START; add exactly one START <command> inside this block",
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
}
