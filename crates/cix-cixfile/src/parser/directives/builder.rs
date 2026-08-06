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
        let command = self.build_template(command, line, source, false)?;
        self.push_builder_command(builder, expected, fetch, line, source, command);
        Ok(())
    }

    pub(in crate::parser) fn push_builder_command(
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
}
