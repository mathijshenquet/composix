use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path};

use crate::model::{
    Assembly, BuildStep, Cixfile, Env, Input, Item, Port, Service, Take, Template, TemplatePart,
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
    paths: Vec<Template>,
    caches: Vec<String>,
    steps: Vec<BuildStep>,
    build_destinations: BTreeSet<String>,
    items: BTreeMap<String, Item>,
    item_destinations: BTreeMap<String, BTreeSet<String>>,
    item_lines: BTreeMap<String, (usize, String)>,
    item_metadata: BTreeMap<String, ServiceMetadata>,
    current_item: Option<String>,
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
        paths: Vec::new(),
        caches: Vec::new(),
        steps: Vec::new(),
        build_destinations: BTreeSet::new(),
        items: BTreeMap::new(),
        item_destinations: BTreeMap::new(),
        item_lines: BTreeMap::new(),
        item_metadata: BTreeMap::new(),
        current_item: None,
    }
    .parse()
}

impl Parser<'_> {
    fn parse(mut self) -> Result<Cixfile, ParseError> {
        while self.index < self.lines.len() {
            let line_number = self.index + 1;
            let source = self.lines[self.index];
            self.index += 1;
            let trimmed = source.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let (directive, arguments) = trimmed
                .split_once(char::is_whitespace)
                .map_or((trimmed, ""), |(directive, arguments)| {
                    (directive, arguments.trim())
                });
            if self.inputs.is_empty()
                && self.items.is_empty()
                && self.steps.is_empty()
                && self.paths.is_empty()
                && self.caches.is_empty()
                && directive != "FROM"
            {
                return Err(ParseError::new(
                    line_number,
                    source,
                    "every Cixfile begins with FROM; try: FROM nixpkgs AS pkgs",
                ));
            }
            match directive {
                "FROM" => self.from(line_number, source, arguments)?,
                "PKG" => return Err(pkg_removed_error(line_number, source, arguments)),
                "PATH" => self.path(line_number, source, arguments)?,
                "COPY" => self.copy(line_number, source, arguments)?,
                "FETCH" | "RUN" => self.build_step(directive, line_number, source, arguments)?,
                "FILE" | "SCRIPT" => self.heredoc(directive, line_number, source, arguments)?,
                "LINK" => self.link(line_number, source, arguments)?,
                "SERVICE" => {
                    return Err(ParseError::new(
                        line_number,
                        source,
                        "SERVICE was renamed to ITEM by D40; use ITEM <name>",
                    ));
                }
                "ITEM" => self.begin_item(line_number, source, arguments)?,
                "TAKE" => self.take(line_number, source, arguments)?,
                "EXEC" => self.exec(line_number, source, arguments, false)?,
                "SETUP" => self.exec(line_number, source, arguments, true)?,
                "ENV" => self.env(line_number, source, arguments)?,
                "PORT" => self.port(line_number, source, arguments)?,
                "LISTENER" => self.listener(line_number, source, arguments)?,
                "CACHE" if self.current_item.is_none() => {
                    self.build_cache(line_number, source, arguments)?
                }
                "STATE" | "CACHE" | "LOGS" | "CONFIG" | "RUNDIR" => {
                    self.directory(directive, line_number, source, arguments)?
                }
                "JIT" => self.jit(line_number, source, arguments)?,
                "OUTBOUND" => self.outbound(line_number, source, arguments)?,
                _ => {
                    return Err(ParseError::new(
                        line_number,
                        source,
                        format!("unknown directive {directive:?}"),
                    ));
                }
            }
        }

        if self.inputs.is_empty() {
            let (line, source) = self
                .lines
                .iter()
                .enumerate()
                .find(|(_, line)| !line.trim().is_empty() && !line.trim().starts_with('#'))
                .map_or((1, ""), |(line, source)| (line + 1, *source));
            return Err(ParseError::new(
                line,
                source,
                "every Cixfile begins with FROM; try: FROM nixpkgs AS pkgs",
            ));
        }
        if self.items.is_empty() {
            let (line, source) = self
                .lines
                .iter()
                .enumerate()
                .find(|(_, line)| !line.trim().is_empty() && !line.trim().starts_with('#'))
                .map_or((1, ""), |(line, source)| (line + 1, *source));
            return Err(ParseError::new(
                line,
                source,
                "a Cixfile must declare at least one ITEM",
            ));
        }
        for (name, item) in &self.items {
            let service = &item.service;
            if service.exec.is_empty() {
                let (line, source) = &self.item_lines[name];
                return Err(ParseError::new(
                    *line,
                    source,
                    format!("ITEM {name:?} must declare exactly one EXEC"),
                ));
            }
            validate_service_references(service, &self.item_metadata[name])?;
            let mut paths = self.paths.clone();
            paths.extend(item.paths.clone());
            validate_bare_commands(service, &self.item_metadata[name], &paths)?;
        }
        if self.steps.is_empty() {
            if let Some(line) = self
                .items
                .values()
                .flat_map(|item| &item.takes)
                .map(|take| take.line)
                .next()
            {
                return Err(ParseError::new(
                    line,
                    self.lines.get(line - 1).copied().unwrap_or_default(),
                    "TAKE requires at least one COPY, FETCH, or RUN step",
                ));
            }
        }

        Ok(Cixfile {
            inputs: self.inputs,
            paths: self.paths,
            caches: self.caches,
            steps: self.steps,
            items: self.items,
        })
    }

    fn from(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        if self.inputs.is_empty()
            && self.items.is_empty()
            && self.steps.is_empty()
            && self.paths.is_empty()
            && self.caches.is_empty()
        {
            // The first meaningful directive is permitted to establish the input universe.
        } else if self.inputs.is_empty() {
            return Err(ParseError::new(
                line,
                source,
                "every Cixfile begins with FROM; try: FROM nixpkgs AS pkgs",
            ));
        } else if !self.items.is_empty()
            || !self.steps.is_empty()
            || !self.paths.is_empty()
            || !self.caches.is_empty()
        {
            return Err(ParseError::new(
                line,
                source,
                "FROM declarations must be contiguous at the beginning of the Cixfile",
            ));
        }
        let fields = arguments.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 3 || fields.get(1) != Some(&"AS") {
            return Err(ParseError::new(
                line,
                source,
                "FROM requires an explicit namespace: FROM <flakeref> AS <name>",
            ));
        }
        let url = normalize_flakeref(fields[0], line, source)?;
        let namespace = fields[2];
        validate_namespace(namespace, line, source)?;
        if self.inputs.contains_key(namespace) {
            return Err(ParseError::new(
                line,
                source,
                format!("FROM namespace {namespace:?} is already declared"),
            ));
        }
        self.inputs.insert(namespace.to_owned(), Input { url });
        Ok(())
    }

    fn path(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        let fields = at_least_one_field(arguments, line, source, "PATH")?;
        let current = self.current_item.clone();
        let conflicts = if let Some(name) = &current {
            self.items[name].service.env.contains_key("PATH")
        } else {
            self.items
                .values()
                .any(|item| item.service.env.contains_key("PATH"))
        };
        if conflicts {
            return Err(ParseError::new(
                line,
                source,
                "PATH conflicts with an explicit ENV PATH declaration",
            ));
        }
        if current.is_none() && !self.steps.is_empty() {
            return Err(ParseError::new(
                line,
                source,
                "PATH must appear before the COPY/FETCH/RUN chain",
            ));
        }
        for field in fields {
            reject_runtime_variable(field, "PATH directory", line, source)?;
            let path = self.build_template(field, line, source, false, false)?;
            validate_path_template(&path, current.is_some(), line, source)?;
            let duplicated = self.paths.iter().any(|existing| existing.same_value(&path))
                || current.as_ref().is_some_and(|name| {
                    self.items[name]
                        .paths
                        .iter()
                        .any(|existing| existing.same_value(&path))
                });
            if duplicated {
                return Err(ParseError::new(
                    line,
                    source,
                    format!("PATH directory {field:?} is duplicated"),
                ));
            }
            let paths = if let Some(name) = &current {
                &mut self.items.get_mut(name).expect("current item exists").paths
            } else {
                &mut self.paths
            };
            paths.push(path);
        }
        Ok(())
    }

    fn build_cache(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        if !self.steps.is_empty() {
            return Err(ParseError::new(
                line,
                source,
                "build CACHE must appear before the COPY/FETCH/RUN chain",
            ));
        }
        let fields = exact_fields(arguments, 1, line, source, "CACHE <dir>")?;
        reject_build_interpolation(fields[0], "CACHE directory", line, source)?;
        reject_runtime_variable(fields[0], "CACHE directory", line, source)?;
        validate_relative_path(fields[0], "CACHE directory", line, source)?;
        let path = Path::new(fields[0]);
        if self.caches.iter().any(|existing| {
            let existing = Path::new(existing);
            path == existing || path.starts_with(existing) || existing.starts_with(path)
        }) {
            return Err(ParseError::new(
                line,
                source,
                format!(
                    "CACHE directory {:?} is duplicated or overlaps another cache",
                    fields[0]
                ),
            ));
        }
        self.caches.push(fields[0].to_owned());
        Ok(())
    }

    fn copy(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        if self.current_item.is_some() {
            return Err(ParseError::new(
                line,
                source,
                "COPY must appear before ITEM blocks",
            ));
        }
        let fields = exact_fields(arguments, 2, line, source, "COPY <src> <dst>")?;
        validate_local_path(fields[0], "COPY source", line, source)?;
        validate_item_path(fields[1], "COPY destination", line, source)?;
        reject_build_interpolation(fields[0], "COPY source", line, source)?;
        reject_build_interpolation(fields[1], "COPY destination", line, source)?;
        reject_runtime_variable(fields[0], "COPY source", line, source)?;
        reject_runtime_variable(fields[1], "COPY destination", line, source)?;
        if !self.build_destinations.insert(fields[1].to_owned()) {
            return Err(ParseError::new(
                line,
                source,
                format!("build destination {:?} is already populated", fields[1]),
            ));
        }
        self.steps.push(BuildStep::Copy {
            src: fields[0].to_owned(),
            dst: fields[1].to_owned(),
            line,
            source: source.to_owned(),
        });
        Ok(())
    }

    fn build_step(
        &mut self,
        directive: &str,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        if self.current_item.is_some() {
            return Err(ParseError::new(
                line,
                source,
                format!("{directive} must appear before ITEM blocks"),
            ));
        }
        if arguments.is_empty() {
            return Err(ParseError::new(
                line,
                source,
                format!("{directive} requires a command"),
            ));
        }
        let command = self.build_template(arguments, line, source, false, false)?;
        let step = if directive == "FETCH" {
            BuildStep::Fetch {
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
        self.steps.push(step);
        Ok(())
    }

    fn heredoc(
        &mut self,
        directive: &str,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        let fields = exact_fields(
            arguments,
            2,
            line,
            source,
            &format!("{directive} <dst> <<EOF"),
        )?;
        validate_item_path(fields[0], &format!("{directive} destination"), line, source)?;
        reject_build_interpolation(fields[0], &format!("{directive} destination"), line, source)?;
        reject_runtime_variable(fields[0], &format!("{directive} destination"), line, source)?;
        let delimiter = fields[1]
            .strip_prefix("<<")
            .filter(|value| !value.is_empty());
        let Some(delimiter) = delimiter else {
            return Err(ParseError::new(
                line,
                source,
                format!("{directive} heredoc must use << followed by a delimiter"),
            ));
        };

        let mut contents = Template { parts: Vec::new() };
        let mut terminated = false;
        while self.index < self.lines.len() {
            let body_line_number = self.index + 1;
            let body_line = self.lines[self.index];
            self.index += 1;
            if body_line == delimiter {
                terminated = true;
                break;
            }
            append_template(
                &mut contents,
                self.build_template(body_line, body_line_number, body_line, true, false)?,
            );
            push_literal(&mut contents, "\n");
        }
        if !terminated {
            return Err(ParseError::new(
                line,
                source,
                format!("unterminated {directive} heredoc; expected {delimiter:?}"),
            ));
        }
        self.claim_item_destination(fields[0], line, source)?;
        let item = if directive == "FILE" {
            Assembly::File {
                dst: fields[0].to_owned(),
                contents,
            }
        } else {
            Assembly::Script {
                dst: fields[0].to_owned(),
                contents,
            }
        };
        self.current_item_mut(directive, line, source)?
            .assembly
            .push(item);
        Ok(())
    }

    fn link(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        let fields = exact_fields(arguments, 2, line, source, "LINK <dst> <target>")?;
        validate_item_path(fields[0], "LINK destination", line, source)?;
        reject_build_interpolation(fields[0], "LINK destination", line, source)?;
        reject_runtime_variable(fields[0], "LINK destination", line, source)?;
        reject_runtime_variable(fields[1], "LINK target", line, source)?;
        let target = self.build_template(fields[1], line, source, false, false)?;
        if target.is_empty() {
            return Err(ParseError::new(
                line,
                source,
                "LINK target must not be empty",
            ));
        }
        self.claim_item_destination(fields[0], line, source)?;
        self.current_item_mut("LINK", line, source)?
            .assembly
            .push(Assembly::Link {
                dst: fields[0].to_owned(),
                target,
            });
        Ok(())
    }

    fn begin_item(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        let fields = exact_fields(arguments, 1, line, source, "ITEM <name>")?;
        let name = fields[0];
        validate_name("item", name, line, source)?;
        if self.items.contains_key(name) {
            return Err(ParseError::new(
                line,
                source,
                format!("ITEM {name:?} is already declared"),
            ));
        }
        self.items.insert(name.to_owned(), Item::empty());
        self.item_destinations
            .insert(name.to_owned(), BTreeSet::new());
        self.item_lines
            .insert(name.to_owned(), (line, source.to_owned()));
        self.item_metadata
            .insert(name.to_owned(), ServiceMetadata::default());
        self.current_item = Some(name.to_owned());
        Ok(())
    }

    fn take(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        let fields = exact_fields(arguments, 2, line, source, "TAKE <build-path> <item-path>")?;
        reject_runtime_variable(fields[0], "TAKE source", line, source)?;
        reject_build_interpolation(fields[1], "TAKE destination", line, source)?;
        reject_runtime_variable(fields[1], "TAKE destination", line, source)?;
        let source_path = self.build_template(fields[0], line, source, false, true)?;
        validate_take_source(&source_path, line, source)?;
        validate_item_path(fields[1], "TAKE destination", line, source)?;
        self.claim_item_destination(fields[1], line, source)?;
        self.current_item_mut("TAKE", line, source)?
            .takes
            .push(Take {
                src: source_path,
                dst: fields[1].to_owned(),
                line,
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
        let fields = at_least_one_field(arguments, line, source, directive)?;
        let templates = fields
            .iter()
            .map(|field| self.build_template(field, line, source, false, false))
            .collect::<Result<Vec<_>, _>>()?;
        let item_name = self.current_item_name(directive, line, source)?.to_owned();
        let service = &mut self
            .items
            .get_mut(&item_name)
            .expect("current item exists")
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
            self.item_metadata
                .get_mut(&item_name)
                .expect("item metadata exists")
                .setup = Some((line, source.to_owned()));
        } else {
            if !service.exec.is_empty() {
                return Err(ParseError::new(
                    line,
                    source,
                    "EXEC is already declared for this service",
                ));
            }
            service.exec = templates;
            service.exec_line = line;
            self.item_metadata
                .get_mut(&item_name)
                .expect("item metadata exists")
                .exec = Some((line, source.to_owned()));
        }
        Ok(())
    }

    fn env(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        let fields = at_least_one_field(arguments, line, source, "ENV")?;
        validate_env_name(fields[0], line, source)?;
        let mut index = 1;
        let default = if fields.get(index) == Some(&"=") {
            index += 1;
            let Some(value) = fields.get(index) else {
                return Err(ParseError::new(
                    line,
                    source,
                    "ENV '=' must be followed by one default value",
                ));
            };
            index += 1;
            reject_runtime_variable(value, "ENV default", line, source)?;
            Some(self.build_template(value, line, source, false, false)?)
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
        let item_has_path = self
            .current_item
            .as_ref()
            .is_some_and(|name| !self.items[name].paths.is_empty());
        if fields[0] == "PATH" && (!self.paths.is_empty() || item_has_path) {
            return Err(ParseError::new(
                line,
                source,
                "ENV PATH conflicts with the item-level PATH directive",
            ));
        }
        let service = self.current_service_mut("ENV", line, source)?;
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
        let item_name = self.current_item_name("PORT", line, source)?.to_owned();
        let service = &mut self
            .items
            .get_mut(&item_name)
            .expect("current item exists")
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
        self.item_metadata
            .get_mut(&item_name)
            .expect("item metadata exists")
            .ports
            .insert(fields[0].to_owned(), (line, source.to_owned()));
        Ok(())
    }

    fn listener(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
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
        let fields = exact_fields(arguments, 1, line, source, &format!("{directive} <path>"))?;
        reject_build_interpolation(fields[0], "directory path", line, source)?;
        reject_runtime_variable(fields[0], "directory path", line, source)?;
        let (root, role) = match directive {
            "STATE" => ("/var/lib", "state"),
            "CACHE" => ("/var/cache", "cache"),
            "LOGS" => ("/var/log", "logs"),
            "CONFIG" => ("/etc", "config"),
            "RUNDIR" => ("/run", "run"),
            _ => unreachable!(),
        };
        validate_role_path(fields[0], root, role, line, source)?;
        let service = self.current_service_mut(directive, line, source)?;
        let paths = match directive {
            "STATE" => &mut service.dirs.state,
            "CACHE" => &mut service.dirs.cache,
            "LOGS" => &mut service.dirs.logs,
            "CONFIG" => &mut service.dirs.config,
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

    fn jit(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        if !arguments.is_empty() {
            return Err(ParseError::new(line, source, "JIT takes no arguments"));
        }
        let service = self.current_service_mut("JIT", line, source)?;
        if service.jit {
            return Err(ParseError::new(
                line,
                source,
                "JIT is already declared for this service",
            ));
        }
        service.jit = true;
        Ok(())
    }

    fn outbound(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        if !arguments.is_empty() {
            return Err(ParseError::new(line, source, "OUTBOUND takes no arguments"));
        }
        let service = self.current_service_mut("OUTBOUND", line, source)?;
        if service.outbound {
            return Err(ParseError::new(
                line,
                source,
                "OUTBOUND is already declared for this item",
            ));
        }
        service.outbound = true;
        Ok(())
    }

    fn current_service_mut(
        &mut self,
        directive: &str,
        line: usize,
        source: &str,
    ) -> Result<&mut Service, ParseError> {
        let name = self.current_item.as_ref().ok_or_else(|| {
            ParseError::new(line, source, format!("{directive} must appear after ITEM"))
        })?;
        Ok(&mut self
            .items
            .get_mut(name)
            .expect("current item exists")
            .service)
    }

    fn current_item_mut(
        &mut self,
        directive: &str,
        line: usize,
        source: &str,
    ) -> Result<&mut Item, ParseError> {
        let name = self.current_item.as_ref().ok_or_else(|| {
            ParseError::new(line, source, format!("{directive} must appear after ITEM"))
        })?;
        Ok(self.items.get_mut(name).expect("current item exists"))
    }

    fn current_item_name(
        &self,
        directive: &str,
        line: usize,
        source: &str,
    ) -> Result<&str, ParseError> {
        self.current_item.as_deref().ok_or_else(|| {
            ParseError::new(line, source, format!("{directive} must appear after ITEM"))
        })
    }

    fn claim_item_destination(
        &mut self,
        destination: &str,
        line: usize,
        source: &str,
    ) -> Result<(), ParseError> {
        let name = self
            .current_item_name("item assembly", line, source)?
            .to_owned();
        if !self
            .item_destinations
            .get_mut(&name)
            .expect("current item destinations exist")
            .insert(destination.to_owned())
        {
            return Err(ParseError::new(
                line,
                source,
                format!("item destination {destination:?} is already populated"),
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
        allow_build: bool,
    ) -> Result<Template, ParseError> {
        build_template(input, line, source, heredoc, allow_build, &self.inputs)
    }
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

fn build_template(
    input: &str,
    line: usize,
    source: &str,
    heredoc: bool,
    allow_build: bool,
    inputs: &BTreeMap<String, Input>,
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
            if reference == "build" {
                if !allow_build {
                    return Err(ParseError::new(
                        line,
                        source,
                        "${build} is only valid in TAKE source position",
                    ));
                }
                if !literal.is_empty() {
                    parts.push(TemplatePart::Literal(std::mem::take(&mut literal)));
                }
                parts.push(TemplatePart::Build { line });
                index = close + 1;
                continue;
            }
            let (namespace, attrpath) = reference.split_once('.').ok_or_else(|| {
                if !reference.contains('.') {
                    let namespace = inputs.keys().next().map(String::as_str).unwrap_or("<namespace>");
                    ParseError::new(
                        line,
                        source,
                        format!("bare build-time interpolation ${{{reference}}}; use ${{{namespace}.{reference}}}"),
                    )
                } else {
                    ParseError::new(
                        line,
                        source,
                        "build-time interpolation must use <namespace>.<attrpath>",
                    )
                }
            })?;
            if !inputs.contains_key(namespace) {
                let declared = inputs.keys().cloned().collect::<Vec<_>>().join(", ");
                return Err(ParseError::new(
                    line,
                    source,
                    format!(
                        "unknown package namespace {namespace:?}; declared namespaces: {declared}"
                    ),
                ));
            }
            if !valid_attrpath(attrpath) {
                return Err(ParseError::new(
                    line,
                    source,
                    "package interpolation must name a dot-separated attribute path after its namespace.",
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
            TemplatePart::Build { line } => target.parts.push(TemplatePart::Build { line }),
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

fn validate_bare_commands(
    service: &Service,
    metadata: &ServiceMetadata,
    paths: &[Template],
) -> Result<(), ParseError> {
    for (arguments, location) in [
        (&service.exec[..], metadata.exec.as_ref()),
        (
            service.setup.as_deref().unwrap_or_default(),
            metadata.setup.as_ref(),
        ),
    ] {
        let Some((line, source)) = location else {
            continue;
        };
        if bare_command(arguments).is_some() && paths.is_empty() {
            return Err(ParseError::new(
                *line,
                source,
            "bare EXEC/SETUP command requires PATH <dir>… or an absolute ${<namespace>.<attrpath>}/bin/... path",
            ));
        }
    }
    Ok(())
}

pub(crate) fn bare_command(arguments: &[Template]) -> Option<String> {
    let command = arguments.first()?.literal_value()?;
    (!command.is_empty() && !command.contains('/') && !command.contains('$')).then_some(command)
}

fn validate_path_template(
    template: &Template,
    item_relative: bool,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    match template.parts.first() {
        Some(TemplatePart::Literal(value)) if value.starts_with('/') => Ok(()),
        Some(TemplatePart::Package { .. }) => Ok(()),
        Some(TemplatePart::Literal(value)) if item_relative => {
            validate_relative_path(value, "ITEM PATH directory", line, source)
        }
        _ => Err(ParseError::new(
            line,
            source,
            "prelude PATH directory must be absolute (for example ${pkgs.coreutils}/bin); relative PATH is item-scoped",
        )),
    }
}

fn validate_take_source(template: &Template, line: usize, source: &str) -> Result<(), ParseError> {
    match template.parts.as_slice() {
        [TemplatePart::Literal(path)] => validate_relative_path(path, "TAKE source", line, source),
        [TemplatePart::Build { .. }] => Ok(()),
        [TemplatePart::Build { .. }, TemplatePart::Literal(path)] if path.starts_with('/') => {
            validate_relative_path(&path[1..], "TAKE source", line, source)
        }
        _ => Err(ParseError::new(
            line,
            source,
            "TAKE source must be a clean build-relative path or ${build}/<path>",
        )),
    }
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

fn normalize_flakeref(value: &str, line: usize, source: &str) -> Result<String, ParseError> {
    if value == "nixpkgs" {
        return Ok(crate::DEFAULT_NIXPKGS_URL.to_owned());
    }
    let github = value.strip_prefix("github:").is_some_and(|path| {
        let parts = path.split('/').collect::<Vec<_>>();
        (2..=3).contains(&parts.len()) && parts.iter().all(|part| !part.is_empty())
    });
    if github || value.starts_with("https://") {
        return Ok(value.to_owned());
    }
    Err(ParseError::new(
        line,
        source,
        "FROM accepts nixpkgs, github:owner/repo[/ref], or an https tarball URL",
    ))
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

fn validate_local_path(
    value: &str,
    label: &str,
    line: usize,
    source: &str,
) -> Result<(), ParseError> {
    validate_relative_path(value, label, line, source)
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
mod tests {
    use super::*;

    const COMPLETE: &str = r#"
# assembly
FROM nixpkgs AS pkgs
CACHE target
COPY index.html www/index.html
ITEM web
TAKE www/index.html www/index.html
FILE etc/app.conf <<CONF
package=${pkgs.nginx}
escaped=$${literal}
runtime=$PORT
CONF
SCRIPT bin/start <<SCRIPT
exec /app/bin/nginx "$PORT"
SCRIPT
LINK bin/nginx ${pkgs.nginx}/bin/nginx

EXEC bin/start $PORT
SETUP bin/start $PORT
ENV PORT = 8080 required secret
PORT http = $PORT
LISTENER admin
STATE /var/lib/web
CACHE /var/cache/web
LOGS /var/log/web
CONFIG /etc/web
RUNDIR /run/web
JIT
"#;

    #[test]
    fn parses_every_v1_directive() {
        let parsed = parse(COMPLETE).unwrap();
        assert_eq!(parsed.caches, ["target"]);
        assert_eq!(parsed.items["web"].assembly.len(), 3);
        assert_eq!(parsed.items["web"].takes.len(), 1);
        let service = &parsed.items["web"].service;
        assert_eq!(service.exec.len(), 2);
        assert!(service.setup.is_some());
        assert_eq!(
            service.env["PORT"],
            Env {
                default: Some(Template::literal("8080")),
                required: true,
                secret: true,
            }
        );
        assert_eq!(service.ports["http"], Port::Env("PORT".into()));
        assert!(service.listeners.contains("admin"));
        assert!(service.jit);

        let Assembly::File { contents, .. } = &parsed.items["web"].assembly[0] else {
            panic!("expected FILE");
        };
        assert!(contents.parts.contains(&TemplatePart::Package {
            namespace: "pkgs".into(),
            attrpath: "nginx".into(),
            line: 9,
        }));
        assert!(contents.literal_value().is_none());
        assert!(contents.parts.iter().any(
            |part| matches!(part, TemplatePart::Literal(value) if value.contains("${literal}") && value.contains("$PORT"))
        ));
    }

    #[test]
    fn parses_fixed_port_and_multiple_items() {
        let parsed =
            parse("FROM nixpkgs AS pkgs\nITEM one\nEXEC bin/one\nPORT http = 8080\nITEM two\nEXEC bin/two\n")
                .unwrap();
        assert_eq!(parsed.items["one"].service.ports["http"], Port::Value(8080));
        assert_eq!(parsed.items.len(), 2);
    }

    #[test]
    fn parses_a_linear_copy_fetch_run_chain_without_losing_shell_syntax() {
        let parsed = parse(
            r#"FROM nixpkgs AS pkgs
PATH ${pkgs.bash}/bin
COPY Cargo.toml Cargo.toml
FETCH cargo fetch --locked
RUN printf '%s\n' "hello world" > result
COPY src.rs src/main.rs
RUN cargo build --release
ITEM app
TAKE target/release/app bin/app
EXEC bin/app
"#,
        )
        .unwrap();
        assert_eq!(parsed.steps.len(), 5);
        assert!(matches!(parsed.steps[0], BuildStep::Copy { .. }));
        let BuildStep::Run { command, line, .. } = &parsed.steps[2] else {
            panic!("expected RUN");
        };
        assert_eq!(*line, 5);
        assert_eq!(
            command.literal_value().as_deref(),
            Some("printf '%s\\n' \"hello world\" > result")
        );
        assert!(matches!(
            parsed.items["app"].takes[0].src.parts[0],
            TemplatePart::Literal(_)
        ));
    }

    #[test]
    fn run_fetch_and_build_interpolation_have_line_numbered_position_errors() {
        for (input, line, message) in [
            (
                "FROM nixpkgs AS pkgs\nRUN\nITEM app\nEXEC /bin/app\n",
                2,
                "requires a command",
            ),
            (
                "FROM nixpkgs AS pkgs\nITEM app\nEXEC /bin/app\nRUN true\n",
                4,
                "before ITEM",
            ),
            (
                "FROM nixpkgs AS pkgs\nRUN echo ${build}\nITEM app\nEXEC /bin/app\n",
                2,
                "only valid in TAKE source position",
            ),
            (
                "FROM nixpkgs AS pkgs\nITEM app\nEXEC ${build}/bin/app\n",
                3,
                "only valid in TAKE source position",
            ),
        ] {
            let error = parse(input).unwrap_err();
            assert_eq!(error.line, line, "{error}");
            assert!(error.message.contains(message), "{error}");
            assert!(error.to_string().contains(&format!("{:?}", error.source)));
        }
    }

    #[test]
    fn from_requires_an_explicit_unique_namespace_before_interpolation() {
        let missing = parse("ITEM app\nEXEC /bin/app\n").unwrap_err();
        assert!(
            missing.message.contains("every Cixfile begins with FROM"),
            "{missing}"
        );

        let missing_as = parse("FROM nixpkgs\nITEM app\nEXEC /bin/app\n").unwrap_err();
        assert!(
            missing_as.message.contains("FROM <flakeref> AS <name>"),
            "{missing_as}"
        );

        let duplicate = parse(
            "FROM nixpkgs AS pkgs\nFROM github:NixOS/nixpkgs/nixos-25.05 AS pkgs\nITEM app\nEXEC /bin/app\n",
        )
        .unwrap_err();
        assert!(
            duplicate.message.contains("already declared"),
            "{duplicate}"
        );

        let unknown = parse(
            "FROM nixpkgs AS pkgs\nITEM app\nLINK bin/app ${stable.hello}/bin/hello\nEXEC bin/app\n",
        )
        .unwrap_err();
        assert!(
            unknown
                .message
                .contains("unknown package namespace \"stable\""),
            "{unknown}"
        );
        assert!(
            unknown.message.contains("declared namespaces: pkgs"),
            "{unknown}"
        );

        let parsed = parse(
            "FROM nixpkgs AS pkgs\nFROM github:NixOS/nixpkgs/nixos-25.05 AS stable\nITEM app\nLINK bin/hello ${stable.hello}/bin/hello\nEXEC bin/hello\n",
        )
        .unwrap();
        assert_eq!(parsed.inputs.len(), 2);
        assert_eq!(
            parsed.inputs["stable"].url,
            "github:NixOS/nixpkgs/nixos-25.05"
        );
    }

    #[test]
    fn path_preserves_declaration_order_and_rejects_duplicates() {
        let parsed =
            parse("FROM nixpkgs AS pkgs\nPATH ${pkgs.first}/bin\nPATH ${pkgs.second}/bin\nITEM app\nEXEC /bin/app\n")
                .unwrap();
        assert_eq!(
            parsed.paths,
            vec![
                Template {
                    parts: vec![
                        TemplatePart::Package {
                            namespace: "pkgs".into(),
                            attrpath: "first".into(),
                            line: 2,
                        },
                        TemplatePart::Literal("/bin".into()),
                    ],
                },
                Template {
                    parts: vec![
                        TemplatePart::Package {
                            namespace: "pkgs".into(),
                            attrpath: "second".into(),
                            line: 3,
                        },
                        TemplatePart::Literal("/bin".into()),
                    ],
                },
            ]
        );

        let error =
            parse("FROM nixpkgs AS pkgs\nPATH ${pkgs.tool}/bin\nPATH ${pkgs.tool}/bin\nITEM app\nEXEC /bin/app\n")
                .unwrap_err();
        assert_eq!(error.line, 3);
        assert!(error.message.contains("duplicated"));
    }

    #[test]
    fn item_paths_are_scoped_and_extend_the_build_path() {
        let parsed = parse(
            "FROM nixpkgs AS pkgs\nPATH ${pkgs.bash}/bin\nITEM api\nPATH bin ${pkgs.coreutils}/bin\nEXEC true\nITEM worker\nPATH ${pkgs.hello}/bin\nEXEC bash\n",
        )
        .unwrap();
        assert_eq!(parsed.paths.len(), 1);
        assert_eq!(parsed.items["api"].paths.len(), 2);
        assert_eq!(parsed.items["worker"].paths.len(), 1);

        let error = parse(
            "FROM nixpkgs AS pkgs\nPATH ${pkgs.bash}/bin\nITEM api\nPATH ${pkgs.bash}/bin\nEXEC bash\n",
        )
        .unwrap_err();
        assert!(error.message.contains("duplicated"), "{error}");
    }

    #[test]
    fn path_rejects_explicit_env_path_and_bare_commands_without_path() {
        for input in [
            "FROM nixpkgs AS pkgs\nPATH /tools\nITEM app\nENV PATH = /other\nEXEC /bin/app\n",
            "FROM nixpkgs AS pkgs\nITEM app\nENV PATH = /other\nPATH /tools\nEXEC /bin/app\n",
        ] {
            let error = parse(input).unwrap_err();
            assert!(error.message.contains("PATH conflicts"), "{error}");
        }

        let error = parse("FROM nixpkgs AS pkgs\nITEM app\nEXEC tool\n").unwrap_err();
        assert_eq!(error.line, 3);
        assert!(error.message.contains("requires PATH"));
    }

    #[test]
    fn all_errors_include_line_and_quoted_source() {
        for (input, line, message) in [
            ("NOPE value\n", 1, "unknown directive"),
            ("PKG nginx\n", 1, "PKG was removed by D32"),
            ("COPY only\nITEM x\nEXEC x\n", 1, "expected COPY"),
            ("FILE x <<EOF\nbody\n", 1, "unterminated FILE heredoc"),
            (
                "LINK x ${missing}\nITEM x\nEXEC x\n",
                1,
                "use ${pkgs.missing}",
            ),
            (
                "ITEM x\nENV BAD-NAME\nEXEC x\n",
                2,
                "environment variable name",
            ),
            ("ITEM x\nEXEC\n", 2, "EXEC requires"),
            (
                "ITEM x\nEXEC bin/x $NOPE\n",
                2,
                "undeclared environment variable",
            ),
            ("ITEM x\nEXEC x\nPORT http = 0\n", 3, "between 1 and 65535"),
            ("ITEM x\nEXEC x\nSTATE /tmp/x\n", 3, "under /var/lib"),
            ("ITEM x\nEXEC x\nJIT yes\n", 3, "takes no arguments"),
            (
                "ITEM x\nEXEC x\nLISTENER http\nLISTENER http\n",
                4,
                "already declared",
            ),
            (
                "COPY $SRC x\nITEM x\nEXEC x\n",
                1,
                "only allowed in EXEC and SETUP",
            ),
            ("ITEM x\nEXEC x\nEXEC y\n", 3, "already declared"),
            ("ITEM x\nEXEC x\nITEM x\n", 3, "already declared"),
        ] {
            let input = format!("FROM nixpkgs AS pkgs\n{input}");
            let error = parse(&input).unwrap_err();
            let rendered = error.to_string();
            assert_eq!(error.line, line + 1, "{input:?}: {rendered}");
            assert!(rendered.contains(message), "{input:?}: {rendered}");
            assert!(
                rendered.contains(&format!("{:?}", error.source)),
                "{input:?}: {rendered}"
            );
        }
    }

    #[test]
    fn interpolation_uses_the_pkgs_namespace_and_accepts_nested_attributes() {
        let parsed = parse(
            "FROM nixpkgs AS pkgs\nITEM x\nLINK bin/black ${pkgs.python3Packages.black}/bin/black\nEXEC bin/black\n",
        )
        .unwrap();
        let Assembly::Link { target, .. } = &parsed.items["x"].assembly[0] else {
            panic!("expected LINK");
        };
        assert_eq!(
            target.parts,
            [
                TemplatePart::Package {
                    namespace: "pkgs".into(),
                    attrpath: "python3Packages.black".into(),
                    line: 3,
                },
                TemplatePart::Literal("/bin/black".into()),
            ]
        );

        let error = parse(
            "FROM nixpkgs AS pkgs\nLINK bin/nginx ${nginx}/bin/nginx\nITEM x\nEXEC bin/nginx\n",
        )
        .unwrap_err();
        assert!(error.message.contains("use ${pkgs.nginx}"), "{error}");
    }

    #[test]
    fn copy_is_never_interpolated() {
        let error =
            parse("FROM nixpkgs AS pkgs\nCOPY ${pkgs.nginx} x\nITEM x\nEXEC x\n").unwrap_err();
        assert!(error
            .message
            .contains("does not support build-time interpolation"));
    }

    #[test]
    fn pkg_directive_explains_the_d32_rewrite() {
        let error =
            parse("FROM nixpkgs AS pkgs\nPKG python3Packages.black\nITEM x\nEXEC x\n").unwrap_err();
        assert_eq!(error.line, 2);
        assert!(
            error
                .message
                .contains("replace ${python3Packages.black} with ${pkgs.python3Packages.black}"),
            "{error}"
        );
    }

    #[test]
    fn rejects_unsafe_item_paths_and_duplicate_destinations() {
        for input in [
            "FROM nixpkgs AS pkgs\nCOPY ../x x\nITEM x\nEXEC x\n",
            "FROM nixpkgs AS pkgs\nITEM x\nFILE / <<E\nx\nE\nEXEC x\n",
            "FROM nixpkgs AS pkgs\nITEM x\nLINK x /target\nFILE x <<E\nx\nE\nEXEC x\n",
        ] {
            assert!(parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn accepts_projected_destinations_and_rejects_d22_denied_paths() {
        let parsed = parse(
            "FROM nixpkgs AS pkgs\nITEM x\nFILE /etc/nginx/nginx.conf <<E\nevents {}\nE\nLINK /srv/www /target\nFILE /cix-probe.conf <<E\nprobe\nE\nEXEC bin/x\n",
        )
        .unwrap();
        assert_eq!(parsed.items["x"].assembly.len(), 3);

        for denied in [
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
            "/",
            "/etc",
            "/usr",
            "/bin",
            "/lib",
            "/lib64",
        ] {
            let input =
                format!("FROM nixpkgs AS pkgs\nITEM x\nFILE {denied} <<E\nx\nE\nEXEC bin/x\n");
            let error = parse(&input).unwrap_err();
            assert!(error.message.contains("D22 v3"), "{denied}: {error}");
        }
    }
}
