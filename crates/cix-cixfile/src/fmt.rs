//! Lossless, line-oriented Cixfile formatting.

use crate::{
    parse, Artifact, Assembly, BuildStep, Builder, Cixfile, Copy, Fetch, Input, NodeCommand,
    ParseError, Service, Template, TemplatePart,
};

/// Formats a Cixfile after first accepting it with the real semantic parser.
pub fn format(input: &str) -> Result<String, ParseError> {
    let mut input = input.replace("\r\n", "\n");
    let parsed = match parse(&input) {
        Ok(parsed) => parsed,
        Err(error) if error.message.contains("is never closed") => {
            input.push_str(if input.ends_with('\n') {
                "}\n"
            } else {
                "\n}\n"
            });
            parse(&input)?
        }
        Err(error) => return Err(error),
    };
    let mut scanner = Scanner::new(&input);
    let entries = scanner.scan();
    let formatted = render(entries);
    debug_assert!(same_semantics(
        &parsed,
        &parse(&formatted).expect("formatter must preserve parsing")
    ));
    Ok(formatted)
}

/// Compares the parsed language while deliberately excluding diagnostic provenance.
pub fn same_semantics(left: &Cixfile, right: &Cixfile) -> bool {
    left.fetch_order == right.fetch_order
        && left.builder_order == right.builder_order
        && left.artifact_order == right.artifact_order
        && left.inputs.len() == right.inputs.len()
        && left.fetches.len() == right.fetches.len()
        && left.builders.len() == right.builders.len()
        && left.artifacts.len() == right.artifacts.len()
        && left.inputs.iter().all(|(name, input)| {
            right
                .inputs
                .get(name)
                .is_some_and(|other| same_input(input, other))
        })
        && left.fetches.iter().all(|(name, fetch)| {
            right
                .fetches
                .get(name)
                .is_some_and(|other| same_fetch(fetch, other))
        })
        && left.builders.iter().all(|(name, builder)| {
            right
                .builders
                .get(name)
                .is_some_and(|other| same_builder(builder, other))
        })
        && left.artifacts.iter().all(|(name, artifact)| {
            right
                .artifacts
                .get(name)
                .is_some_and(|other| same_artifact(artifact, other))
        })
}

fn same_input(left: &Input, right: &Input) -> bool {
    left.url == right.url && left.kind == right.kind && left.overlays == right.overlays
}

fn same_fetch(left: &Fetch, right: &Fetch) -> bool {
    left.expected == right.expected && same_node_command(&left.command, &right.command)
}

fn same_builder(left: &Builder, right: &Builder) -> bool {
    left.imports.len() == right.imports.len()
        && left
            .imports
            .iter()
            .zip(&right.imports)
            .all(|(left, right)| same_template(left, right))
        && left.steps.len() == right.steps.len()
        && left
            .steps
            .iter()
            .zip(&right.steps)
            .all(|(left, right)| match (left, right) {
                (
                    BuildStep::Env {
                        name: left_name,
                        value: left_value,
                        ..
                    },
                    BuildStep::Env {
                        name: right_name,
                        value: right_value,
                        ..
                    },
                ) => left_name == right_name && same_template(left_value, right_value),
                (BuildStep::Copy(left), BuildStep::Copy(right)) => same_copy(left, right),
                (
                    BuildStep::Fetch {
                        expected: left_expected,
                        command: left_command,
                        ..
                    },
                    BuildStep::Fetch {
                        expected: right_expected,
                        command: right_command,
                        ..
                    },
                ) => {
                    left_expected == right_expected
                        && same_node_command(left_command, right_command)
                }
                (
                    BuildStep::Run {
                        command: left_command,
                        ..
                    },
                    BuildStep::Run {
                        command: right_command,
                        ..
                    },
                ) => same_node_command(left_command, right_command),
                _ => false,
            })
}

fn same_node_command(left: &NodeCommand, right: &NodeCommand) -> bool {
    match (left, right) {
        (NodeCommand::Legacy(left), NodeCommand::Legacy(right)) => same_template(left, right),
        (NodeCommand::Argv(left), NodeCommand::Argv(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| same_template(left, right))
        }
        (
            NodeCommand::Heredoc {
                interpreter: left_interpreter,
                body: left_body,
            },
            NodeCommand::Heredoc {
                interpreter: right_interpreter,
                body: right_body,
            },
        ) => {
            same_template(left_interpreter, right_interpreter)
                && same_template(left_body, right_body)
        }
        _ => false,
    }
}

fn same_artifact(left: &Artifact, right: &Artifact) -> bool {
    left.kind == right.kind
        && left.imports.len() == right.imports.len()
        && left
            .imports
            .iter()
            .zip(&right.imports)
            .all(|(left, right)| same_template(left, right))
        && left.copies.len() == right.copies.len()
        && left
            .copies
            .iter()
            .zip(&right.copies)
            .all(|(left, right)| same_copy(left, right))
        && left.assembly.len() == right.assembly.len()
        && left
            .assembly
            .iter()
            .zip(&right.assembly)
            .all(|(left, right)| match (left, right) {
                (
                    Assembly::File {
                        dst: left_dst,
                        contents: left_contents,
                        ..
                    },
                    Assembly::File {
                        dst: right_dst,
                        contents: right_contents,
                        ..
                    },
                ) => left_dst == right_dst && same_template(left_contents, right_contents),
                (
                    Assembly::Link {
                        dst: left_dst,
                        target: left_target,
                        ..
                    },
                    Assembly::Link {
                        dst: right_dst,
                        target: right_target,
                        ..
                    },
                ) => left_dst == right_dst && same_template(left_target, right_target),
                _ => false,
            })
        && same_service(&left.service, &right.service)
}

fn same_copy(left: &Copy, right: &Copy) -> bool {
    left.dst == right.dst && left.mode == right.mode && same_template(&left.src, &right.src)
}

fn same_service(left: &Service, right: &Service) -> bool {
    left.start.len() == right.start.len()
        && left
            .start
            .iter()
            .zip(&right.start)
            .all(|(left, right)| same_template(left, right))
        && match (&left.start_pre, &right.start_pre) {
            (Some(left), Some(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right)
                        .all(|(left, right)| same_template(left, right))
            }
            (None, None) => true,
            _ => false,
        }
        && left.env.len() == right.env.len()
        && left.env.iter().all(|(name, left)| {
            right.env.get(name).is_some_and(|right| {
                left.required == right.required
                    && left.secret == right.secret
                    && match (&left.default, &right.default) {
                        (Some(left), Some(right)) => same_template(left, right),
                        (None, None) => true,
                        _ => false,
                    }
            })
        })
        && left.ports == right.ports
        && left.listeners == right.listeners
        && left.readiness == right.readiness
        && left.liveness == right.liveness
        && left.secrets == right.secrets
        && left.dirs == right.dirs
        && left.claims == right.claims
        && left.shm == right.shm
        && left.stop_signal == right.stop_signal
}

fn same_template(left: &Template, right: &Template) -> bool {
    left.parts.len() == right.parts.len()
        && left
            .parts
            .iter()
            .zip(&right.parts)
            .all(|(left, right)| match (left, right) {
                (TemplatePart::Literal(left), TemplatePart::Literal(right)) => left == right,
                (
                    TemplatePart::Package {
                        namespace: left_namespace,
                        attrpath: left_attrpath,
                        ..
                    },
                    TemplatePart::Package {
                        namespace: right_namespace,
                        attrpath: right_attrpath,
                        ..
                    },
                ) => left_namespace == right_namespace && left_attrpath == right_attrpath,
                (
                    TemplatePart::Binder {
                        name: left_name, ..
                    },
                    TemplatePart::Binder {
                        name: right_name, ..
                    },
                ) => left_name == right_name,
                (
                    TemplatePart::InputMetadata {
                        namespace: left_namespace,
                        attribute: left_attribute,
                        ..
                    },
                    TemplatePart::InputMetadata {
                        namespace: right_namespace,
                        attribute: right_attribute,
                        ..
                    },
                ) => left_namespace == right_namespace && left_attribute == right_attribute,
                _ => false,
            })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Blank,
    Comment,
    Prelude,
    BlockHeader,
    Body,
    HeredocBody,
    HeredocTerminator,
    Closing,
}

struct Entry {
    kind: Kind,
    text: String,
}

struct Scanner<'a> {
    lines: Vec<&'a str>,
    index: usize,
}

impl<'a> Scanner<'a> {
    fn new(input: &'a str) -> Self {
        let mut lines = input.split('\n').collect::<Vec<_>>();
        if input.ends_with('\n') {
            lines.pop();
        }
        Self { lines, index: 0 }
    }

    fn scan(&mut self) -> Vec<Entry> {
        let mut entries = Vec::new();
        let mut in_block = false;
        while self.index < self.lines.len() {
            let line = self.lines[self.index];
            self.index += 1;
            if line.trim().is_empty() {
                entries.push(Entry {
                    kind: Kind::Blank,
                    text: String::new(),
                });
                continue;
            }
            if line.trim_start().starts_with('#') {
                entries.push(Entry {
                    kind: Kind::Comment,
                    text: line.to_owned(),
                });
                continue;
            }
            if line.trim() == "}" {
                entries.push(Entry {
                    kind: Kind::Closing,
                    text: "}".into(),
                });
                in_block = false;
                continue;
            }

            let start = entries.len();
            let mut physical = vec![line];
            while physical
                .last()
                .is_some_and(|current| current.trim_end().ends_with('\\'))
                && self.index < self.lines.len()
            {
                physical.push(self.lines[self.index]);
                self.index += 1;
            }
            let logical = logical_line(&physical);
            let (directive, arguments) = split_directive(&logical);
            let kind = match directive {
                "FROM" => Kind::Prelude,
                "FETCH" if !in_block => Kind::Prelude,
                "BUILDER" | "SERVICE" | "APP" | "ITEM" => Kind::BlockHeader,
                _ => Kind::Body,
            };
            if kind == Kind::BlockHeader {
                in_block = true;
            }
            for (offset, source) in physical.iter().enumerate() {
                entries.push(Entry {
                    kind,
                    text: format_directive_line(source, directive, kind, offset > 0),
                });
            }
            if let Some(delimiter) = heredoc_delimiter(directive, arguments) {
                while self.index < self.lines.len() {
                    let body = self.lines[self.index];
                    self.index += 1;
                    let kind = if body == delimiter {
                        Kind::HeredocTerminator
                    } else {
                        Kind::HeredocBody
                    };
                    entries.push(Entry {
                        kind,
                        text: body.to_owned(),
                    });
                    if kind == Kind::HeredocTerminator {
                        break;
                    }
                }
            }
            debug_assert!(entries.len() > start);
        }
        entries
    }
}

fn logical_line(lines: &[&str]) -> String {
    let mut logical = lines[0].trim_end().to_owned();
    for line in &lines[1..] {
        logical.pop();
        logical.truncate(logical.trim_end().len());
        logical.push(' ');
        logical.push_str(line.trim());
    }
    logical
}

fn split_directive(line: &str) -> (&str, &str) {
    let trimmed = line.trim();
    trimmed
        .split_once(char::is_whitespace)
        .map_or((trimmed, ""), |(directive, arguments)| {
            (directive, arguments.trim())
        })
}

fn heredoc_delimiter<'a>(directive: &str, arguments: &'a str) -> Option<&'a str> {
    let fields = arguments.split_whitespace().collect::<Vec<_>>();
    match directive {
        "RUN" if fields.len() == 1 => fields[0].strip_prefix("<<"),
        "RUN" if fields.len() == 2 => fields[1].strip_prefix("<<"),
        "FILE" if fields.len() == 2 => fields[1].strip_prefix("<<"),
        _ => None,
    }
    .filter(|delimiter| !delimiter.is_empty())
}

fn format_directive_line(source: &str, directive: &str, kind: Kind, continuation: bool) -> String {
    let indent = match kind {
        Kind::Prelude | Kind::BlockHeader | Kind::Closing => 0,
        Kind::Body => 2,
        _ => 0,
    } + usize::from(continuation) * 4;
    let text = source.trim();
    let text = if continuation || matches!(directive, "RUN" | "FETCH") {
        text.to_owned()
    } else {
        normalize_tokens(text)
    };
    format!("{}{}", " ".repeat(indent), text)
}

fn normalize_tokens(text: &str) -> String {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    for character in text.chars() {
        if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
            word.push(character);
        } else if character.is_whitespace() && quote.is_none() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(character);
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words.join(" ")
}

fn render(entries: Vec<Entry>) -> String {
    let mut output = Vec::new();
    for entry in entries {
        if entry.kind == Kind::Blank {
            continue;
        }
        if entry.kind == Kind::BlockHeader
            && !output.is_empty()
            && output.last().is_some_and(String::is_empty)
        {
            // A previous header already established this boundary.
        } else if entry.kind == Kind::BlockHeader && !output.is_empty() {
            output.push(String::new());
        }
        output.push(entry.text);
    }
    while output.last().is_some_and(String::is_empty) {
        output.pop();
    }
    format!("{}\n", output.join("\n"))
}
