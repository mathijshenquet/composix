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
    pub(super) lets: BTreeMap<String, Vec<String>>,
    pub(super) args: BTreeMap<String, Arg>,
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
    pub(super) opened_block: Option<OpenedBlock>,
}

#[derive(Clone)]
pub(super) struct OpenedBlock {
    pub(super) kind: &'static str,
    pub(super) name: String,
    pub(super) line: usize,
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
        lets: BTreeMap::new(),
        args: BTreeMap::new(),
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
        opened_block: None,
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
            if trimmed == "}" {
                self.close_block(line_number, source)?;
                continue;
            }
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
                "LET" => self.let_binding(line_number, source, arguments)?,
                "ARG" => self.arg_binding(line_number, source, arguments)?,
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
                           "PATH was removed; use ENV PATH=<value>; see docs/cixfile.md#runtime-path"
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
                "LINK" => {
                    return Err(ParseError::new(
                        line_number,
                        source,
                        "LINK was removed; use COPY <source> <absolute-destination>; see docs/cixfile.md#link",
                    ));
                }
                "START" => self.start(line_number, source, arguments, false)?,
                "START_PRE" => self.start(line_number, source, arguments, true)?,
                "ENV" => self.env(line_number, source, arguments)?,
                "SECRET" => self.secret(line_number, source, arguments)?,
                "PORT" => self.port(line_number, source, arguments)?,
                "LISTENER" => self.listener(line_number, source, arguments)?,
                "READINESS" => self.health_probe(line_number, source, arguments, true)?,
                "LIVENESS" => self.health_probe(line_number, source, arguments, false)?,
                "SHM" => self.shm(line_number, source, arguments)?,
                "STOPSIGNAL" => self.stop_signal(line_number, source, arguments)?,
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

        if let Some(opened) = &self.opened_block {
            return Err(ParseError::new(
                opened.line,
                self.lines.get(opened.line - 1).copied().unwrap_or_default(),
                format!(
                    "{} {} opened at line {} is never closed",
                    opened.kind, opened.name, opened.line
                ),
            ));
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
        materialize_structural_copy_ancestors(&mut self.artifacts);
        Ok(Cixfile {
            lets: self.lets,
            args: self.args,
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

impl Parser<'_> {
    fn close_block(&mut self, line: usize, source: &str) -> Result<(), ParseError> {
        if self.opened_block.is_none() {
            return Err(ParseError::new(
                line,
                source,
                "unexpected }; phase blocks do not nest",
            ));
        }
        self.opened_block = None;
        self.current = None;
        Ok(())
    }
}

fn materialize_structural_copy_ancestors(artifacts: &mut BTreeMap<String, Artifact>) {
    for artifact in artifacts.values_mut() {
        let role_paths = artifact
            .service
            .dirs
            .state
            .iter()
            .chain(&artifact.service.dirs.cache)
            .chain(&artifact.service.dirs.logs)
            .chain(&artifact.service.dirs.config)
            .chain(&artifact.service.dirs.run)
            .chain(artifact.service.dirs.data.keys())
            .filter_map(|path| path.strip_prefix('/'))
            .collect::<Vec<_>>();
        let later_writes = artifact
            .copies
            .iter()
            .map(|copy| (copy.line, copy.dst.clone()))
            .chain(artifact.assembly.iter().map(|assembly| match assembly {
                Assembly::File { dst, line, .. } | Assembly::Link { dst, line, .. } => {
                    (*line, dst.clone())
                }
            }))
            .collect::<Vec<_>>();
        for copy in &mut artifact.copies {
            if copy.mode == CopyMode::Materialize {
                continue;
            }
            let destination = copy.dst.as_str();
            let has_role_mount = role_paths
                .iter()
                .any(|path| path_is_at_or_beneath(path, destination));
            let has_later_write = later_writes.iter().any(|(line, path)| {
                *line > copy.line && path_is_strictly_beneath(path, destination)
            });
            if destination == "." || has_role_mount || has_later_write {
                copy.mode = CopyMode::Materialize;
            }
        }
    }
}

fn path_is_at_or_beneath(path: &str, ancestor: &str) -> bool {
    ancestor == "."
        || path == ancestor
        || path
            .strip_prefix(ancestor)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn path_is_strictly_beneath(path: &str, ancestor: &str) -> bool {
    path != ancestor && path_is_at_or_beneath(path, ancestor)
}
