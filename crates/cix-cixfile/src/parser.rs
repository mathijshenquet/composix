use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path};

use crate::model::{Cixfile, Env, Item, Port, Service, Template, TemplatePart};

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
    packages: BTreeSet<String>,
    items: Vec<Item>,
    destinations: BTreeSet<String>,
    services: BTreeMap<String, Service>,
    service_lines: BTreeMap<String, (usize, String)>,
    service_metadata: BTreeMap<String, ServiceMetadata>,
    current_service: Option<String>,
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
        packages: BTreeSet::new(),
        items: Vec::new(),
        destinations: BTreeSet::new(),
        services: BTreeMap::new(),
        service_lines: BTreeMap::new(),
        service_metadata: BTreeMap::new(),
        current_service: None,
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
            match directive {
                "PKG" => self.package(line_number, source, arguments)?,
                "COPY" => self.copy(line_number, source, arguments)?,
                "FILE" | "SCRIPT" => self.heredoc(directive, line_number, source, arguments)?,
                "LINK" => self.link(line_number, source, arguments)?,
                "SERVICE" => self.begin_service(line_number, source, arguments)?,
                "EXEC" => self.exec(line_number, source, arguments, false)?,
                "SETUP" => self.exec(line_number, source, arguments, true)?,
                "ENV" => self.env(line_number, source, arguments)?,
                "PORT" => self.port(line_number, source, arguments)?,
                "LISTENER" => self.listener(line_number, source, arguments)?,
                "STATE" | "CACHE" | "LOGS" | "CONFIG" | "RUNDIR" => {
                    self.directory(directive, line_number, source, arguments)?
                }
                "JIT" => self.jit(line_number, source, arguments)?,
                _ => {
                    return Err(ParseError::new(
                        line_number,
                        source,
                        format!("unknown directive {directive:?}"),
                    ));
                }
            }
        }

        if self.services.is_empty() {
            let (line, source) = self
                .lines
                .iter()
                .enumerate()
                .find(|(_, line)| !line.trim().is_empty() && !line.trim().starts_with('#'))
                .map_or((1, ""), |(line, source)| (line + 1, *source));
            return Err(ParseError::new(
                line,
                source,
                "a Cixfile must declare at least one SERVICE",
            ));
        }
        for (name, service) in &self.services {
            if service.exec.is_empty() {
                let (line, source) = &self.service_lines[name];
                return Err(ParseError::new(
                    *line,
                    source,
                    format!("SERVICE {name:?} must declare exactly one EXEC"),
                ));
            }
            validate_service_references(service, &self.service_metadata[name])?;
        }

        Ok(Cixfile {
            packages: self.packages,
            items: self.items,
            services: self.services,
        })
    }

    fn package(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        let fields = exact_fields(arguments, 1, line, source, "PKG <attr>")?;
        let attribute = fields[0];
        if !valid_package_attr(attribute) {
            return Err(ParseError::new(
                line,
                source,
                "PKG attribute must be a dot-separated Nix attribute path",
            ));
        }
        if !self.packages.insert(attribute.to_owned()) {
            return Err(ParseError::new(
                line,
                source,
                format!("package {attribute:?} is already declared"),
            ));
        }
        Ok(())
    }

    fn copy(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        let fields = exact_fields(arguments, 2, line, source, "COPY <src> <dst>")?;
        validate_local_path(fields[0], "COPY source", line, source)?;
        validate_item_path(fields[1], "COPY destination", line, source)?;
        reject_build_interpolation(fields[0], "COPY source", line, source)?;
        reject_build_interpolation(fields[1], "COPY destination", line, source)?;
        reject_runtime_variable(fields[0], "COPY source", line, source)?;
        reject_runtime_variable(fields[1], "COPY destination", line, source)?;
        self.claim_destination(fields[1], line, source)?;
        self.items.push(Item::Copy {
            src: fields[0].to_owned(),
            dst: fields[1].to_owned(),
        });
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
                build_template(body_line, &self.packages, body_line_number, body_line, true)?,
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
        self.claim_destination(fields[0], line, source)?;
        let item = if directive == "FILE" {
            Item::File {
                dst: fields[0].to_owned(),
                contents,
            }
        } else {
            Item::Script {
                dst: fields[0].to_owned(),
                contents,
            }
        };
        self.items.push(item);
        Ok(())
    }

    fn link(&mut self, line: usize, source: &str, arguments: &str) -> Result<(), ParseError> {
        let fields = exact_fields(arguments, 2, line, source, "LINK <dst> <target>")?;
        validate_item_path(fields[0], "LINK destination", line, source)?;
        reject_build_interpolation(fields[0], "LINK destination", line, source)?;
        reject_runtime_variable(fields[0], "LINK destination", line, source)?;
        reject_runtime_variable(fields[1], "LINK target", line, source)?;
        let target = build_template(fields[1], &self.packages, line, source, false)?;
        if target.is_empty() {
            return Err(ParseError::new(
                line,
                source,
                "LINK target must not be empty",
            ));
        }
        self.claim_destination(fields[0], line, source)?;
        self.items.push(Item::Link {
            dst: fields[0].to_owned(),
            target,
        });
        Ok(())
    }

    fn begin_service(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        let fields = exact_fields(arguments, 1, line, source, "SERVICE <name>")?;
        let name = fields[0];
        validate_name("service", name, line, source)?;
        if self.services.contains_key(name) {
            return Err(ParseError::new(
                line,
                source,
                format!("SERVICE {name:?} is already declared"),
            ));
        }
        self.services.insert(name.to_owned(), Service::empty());
        self.service_lines
            .insert(name.to_owned(), (line, source.to_owned()));
        self.service_metadata
            .insert(name.to_owned(), ServiceMetadata::default());
        self.current_service = Some(name.to_owned());
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
            .map(|field| build_template(field, &self.packages, line, source, false))
            .collect::<Result<Vec<_>, _>>()?;
        let service_name = self
            .current_service_name(directive, line, source)?
            .to_owned();
        let service = self
            .services
            .get_mut(&service_name)
            .expect("current service exists");
        if setup {
            if service.setup.is_some() {
                return Err(ParseError::new(
                    line,
                    source,
                    "SETUP is already declared for this service",
                ));
            }
            service.setup = Some(templates);
            self.service_metadata
                .get_mut(&service_name)
                .expect("service metadata exists")
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
            self.service_metadata
                .get_mut(&service_name)
                .expect("service metadata exists")
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
            Some(build_template(value, &self.packages, line, source, false)?)
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
        let service_name = self.current_service_name("PORT", line, source)?.to_owned();
        let service = self
            .services
            .get_mut(&service_name)
            .expect("current service exists");
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
        self.service_metadata
            .get_mut(&service_name)
            .expect("service metadata exists")
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

    fn current_service_mut(
        &mut self,
        directive: &str,
        line: usize,
        source: &str,
    ) -> Result<&mut Service, ParseError> {
        let name = self.current_service.as_ref().ok_or_else(|| {
            ParseError::new(
                line,
                source,
                format!("{directive} must appear after SERVICE"),
            )
        })?;
        Ok(self.services.get_mut(name).expect("current service exists"))
    }

    fn current_service_name(
        &self,
        directive: &str,
        line: usize,
        source: &str,
    ) -> Result<&str, ParseError> {
        self.current_service.as_deref().ok_or_else(|| {
            ParseError::new(
                line,
                source,
                format!("{directive} must appear after SERVICE"),
            )
        })
    }

    fn claim_destination(
        &mut self,
        destination: &str,
        line: usize,
        source: &str,
    ) -> Result<(), ParseError> {
        if !self.destinations.insert(destination.to_owned()) {
            return Err(ParseError::new(
                line,
                source,
                format!("item destination {destination:?} is already populated"),
            ));
        }
        Ok(())
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
    packages: &BTreeSet<String>,
    line: usize,
    source: &str,
    heredoc: bool,
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
                    "unterminated ${…} package interpolation",
                ));
            };
            let close = index + 2 + close_offset;
            let name = &input[index + 2..close];
            if !packages.contains(name) {
                return Err(ParseError::new(
                    line,
                    source,
                    format!("unknown package interpolation ${{{name}}}; declare it with PKG"),
                ));
            }
            if !literal.is_empty() {
                parts.push(TemplatePart::Literal(std::mem::take(&mut literal)));
            }
            parts.push(TemplatePart::Package(name.to_owned()));
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
            TemplatePart::Package(package) => target.parts.push(TemplatePart::Package(package)),
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
            format!("{label} does not support package interpolation"),
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

fn valid_package_attr(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(valid_package_component)
}

fn valid_package_component(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|value| value.is_ascii_alphabetic() || value == b'_')
        && bytes.all(|value| value.is_ascii_alphanumeric() || matches!(value, b'_' | b'-' | b'\''))
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
PKG nginx
COPY index.html www/index.html
FILE etc/app.conf <<CONF
package=${nginx}
escaped=$${literal}
runtime=$PORT
CONF
SCRIPT bin/start <<SCRIPT
exec /app/bin/nginx "$PORT"
SCRIPT
LINK bin/nginx ${nginx}/bin/nginx

SERVICE web
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
        assert_eq!(parsed.packages, BTreeSet::from(["nginx".to_owned()]));
        assert_eq!(parsed.items.len(), 4);
        let service = &parsed.services["web"];
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

        let Item::File { contents, .. } = &parsed.items[1] else {
            panic!("expected FILE");
        };
        assert!(contents
            .parts
            .contains(&TemplatePart::Package("nginx".into())));
        assert!(contents.literal_value().is_none());
        assert!(contents.parts.iter().any(
            |part| matches!(part, TemplatePart::Literal(value) if value.contains("${literal}") && value.contains("$PORT"))
        ));
    }

    #[test]
    fn parses_fixed_port_and_multiple_services() {
        let parsed =
            parse("SERVICE one\nEXEC bin/one\nPORT http = 8080\nSERVICE two\nEXEC bin/two\n")
                .unwrap();
        assert_eq!(parsed.services["one"].ports["http"], Port::Value(8080));
        assert_eq!(parsed.services.len(), 2);
    }

    #[test]
    fn all_errors_include_line_and_quoted_source() {
        for (input, line, message) in [
            ("NOPE value\n", 1, "unknown directive"),
            ("PKG\n", 1, "expected PKG"),
            ("COPY only\nSERVICE x\nEXEC x\n", 1, "expected COPY"),
            ("FILE x <<EOF\nbody\n", 1, "unterminated FILE heredoc"),
            (
                "LINK x ${missing}\nSERVICE x\nEXEC x\n",
                1,
                "unknown package interpolation",
            ),
            (
                "SERVICE x\nENV BAD-NAME\nEXEC x\n",
                2,
                "environment variable name",
            ),
            ("SERVICE x\nEXEC\n", 2, "EXEC requires"),
            (
                "SERVICE x\nEXEC bin/x $NOPE\n",
                2,
                "undeclared environment variable",
            ),
            (
                "SERVICE x\nEXEC x\nPORT http = 0\n",
                3,
                "between 1 and 65535",
            ),
            ("SERVICE x\nEXEC x\nSTATE /tmp/x\n", 3, "under /var/lib"),
            ("SERVICE x\nEXEC x\nJIT yes\n", 3, "takes no arguments"),
            (
                "SERVICE x\nEXEC x\nLISTENER http\nLISTENER http\n",
                4,
                "already declared",
            ),
            (
                "COPY $SRC x\nSERVICE x\nEXEC x\n",
                1,
                "only allowed in EXEC and SETUP",
            ),
            ("SERVICE x\nEXEC x\nEXEC y\n", 3, "already declared"),
            ("SERVICE x\nEXEC x\nSERVICE x\n", 3, "already declared"),
        ] {
            let error = parse(input).unwrap_err();
            let rendered = error.to_string();
            assert_eq!(error.line, line, "{input:?}: {rendered}");
            assert!(rendered.contains(message), "{input:?}: {rendered}");
            assert!(
                rendered.contains(&format!("{:?}", error.source)),
                "{input:?}: {rendered}"
            );
        }
    }

    #[test]
    fn rejects_interpolation_before_package_declaration() {
        let error =
            parse("LINK bin/nginx ${nginx}/bin/nginx\nPKG nginx\nSERVICE x\nEXEC x\n").unwrap_err();
        assert!(error.message.contains("declare it with PKG"));
    }

    #[test]
    fn copy_is_never_interpolated() {
        let error = parse("PKG nginx\nCOPY ${nginx} x\nSERVICE x\nEXEC x\n").unwrap_err();
        assert!(error
            .message
            .contains("does not support package interpolation"));
    }

    #[test]
    fn rejects_unsafe_item_paths_and_duplicate_destinations() {
        for input in [
            "COPY ../x x\nSERVICE x\nEXEC x\n",
            "FILE / <<E\nx\nE\nSERVICE x\nEXEC x\n",
            "COPY a x\nLINK x /target\nSERVICE x\nEXEC x\n",
        ] {
            assert!(parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn accepts_projected_destinations_and_rejects_d22_denied_paths() {
        let parsed = parse(
            "FILE /etc/nginx/nginx.conf <<E\nevents {}\nE\nLINK /srv/www /target\nFILE /cix-probe.conf <<E\nprobe\nE\nSERVICE x\nEXEC bin/x\n",
        )
        .unwrap();
        assert_eq!(parsed.items.len(), 3);

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
            let input = format!("FILE {denied} <<E\nx\nE\nSERVICE x\nEXEC bin/x\n");
            let error = parse(&input).unwrap_err();
            assert!(error.message.contains("D22 v3"), "{denied}: {error}");
        }
    }
}
