use std::collections::BTreeSet;

use crate::*;

use super::super::machine::{CurrentBlock, ParseError, Parser};
use super::super::validate::*;

impl Parser<'_> {
    pub(in crate::parser) fn begin_builder(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        let (arguments, opened) = phase_header(arguments, "BUILDER", line, source)?;
        if self.opened_block.is_some() {
            return Err(ParseError::new(
                line,
                source,
                "phase blocks are single-level; close the current block with } first",
            ));
        }
        let fields = exact_fields(arguments, 1, line, source, "BUILDER <name> {")?;
        let name = fields[0];
        validate_name("builder", name, line, source)?;
        self.declare_name(name, "BUILDER block", line, source)?;
        self.builders.insert(name.to_owned(), Builder::empty(line));
        self.builder_order.push(name.to_owned());
        self.destinations.insert(name.to_owned(), BTreeSet::new());
        self.current = Some(CurrentBlock::Builder(name.to_owned()));
        if opened {
            self.opened_block = Some(super::super::machine::OpenedBlock {
                kind: "BUILDER",
                name: name.to_owned(),
                line,
            });
        }
        Ok(())
    }

    pub(in crate::parser) fn run(
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
        let command = if self.opened_block.is_none() && arguments.starts_with("<<") {
            let delimiter =
                heredoc_delimiter(arguments, "RUN", line, source)?.expect("legacy delimiter");
            NodeCommand::Legacy(self.read_heredoc_body("RUN", delimiter, line, source)?)
        } else if let Some((interpreter, delimiter)) = run_heredoc(arguments, line, source)? {
            NodeCommand::Heredoc {
                interpreter: self.build_template(&interpreter, line, source, false)?,
                body: self.read_heredoc_body("RUN", &delimiter, line, source)?,
            }
        } else {
            if arguments.is_empty() {
                return Err(ParseError::new(line, source, "RUN requires a command"));
            }
            if self.opened_block.is_none() && legacy_shell_form(arguments) {
                NodeCommand::Legacy(self.build_template(arguments, line, source, false)?)
            } else {
                let argv = argv_fields(arguments, line, source, "RUN")?;
                reject_shell_variable(&argv, line, source)?;
                NodeCommand::Argv(self.build_argv_templates(argv, line, source)?)
            }
        };
        let (environment, ignored_evidence, expected) = self.node_clauses(line, source)?;
        if expected.is_some() {
            return Err(ParseError::new(
                line,
                source,
                "EXPECT is only valid on FETCH nodes",
            ));
        }
        self.push_builder_command(
            &name,
            None,
            false,
            line,
            source,
            command,
            environment,
            ignored_evidence,
        );
        Ok(())
    }

    pub(in crate::parser) fn builder_command(
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
        let command = if let Some((interpreter, delimiter)) = run_heredoc(command, line, source)? {
            NodeCommand::Heredoc {
                interpreter: self.build_template(&interpreter, line, source, false)?,
                body: self.read_heredoc_body(directive, &delimiter, line, source)?,
            }
        } else {
            if self.opened_block.is_none() && legacy_shell_form(command) {
                NodeCommand::Legacy(self.build_template(command, line, source, false)?)
            } else {
                let argv = argv_fields(command, line, source, directive)?;
                reject_shell_variable(&argv, line, source)?;
                NodeCommand::Argv(self.build_argv_templates(argv, line, source)?)
            }
        };
        let (environment, ignored_evidence, clause_expected) = self.node_clauses(line, source)?;
        let expected = clause_expected.or(expected);
        self.push_builder_command(
            builder,
            expected,
            fetch,
            line,
            source,
            command,
            environment,
            ignored_evidence,
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::parser) fn push_builder_command(
        &mut self,
        builder: &str,
        expected: Option<String>,
        fetch: bool,
        line: usize,
        source: &str,
        command: NodeCommand,
        environment: std::collections::BTreeMap<String, Template>,
        ignored_evidence: std::collections::BTreeSet<String>,
    ) {
        let step = if fetch {
            BuildStep::Fetch {
                expected,
                command,
                environment,
                ignored_evidence,
                line,
                source: source.to_owned(),
            }
        } else {
            BuildStep::Run {
                command,
                environment,
                ignored_evidence,
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
}

fn legacy_shell_form(arguments: &str) -> bool {
    arguments.contains(['|', '&', ';', '>', '<', '`']) || arguments.contains('$')
}
