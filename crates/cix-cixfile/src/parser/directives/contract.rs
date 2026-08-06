use std::collections::BTreeSet;

use crate::*;

use super::super::machine::{CurrentBlock, ParseError, Parser, ServiceMetadata};
use super::super::validate::*;

impl Parser<'_> {
    pub(in crate::parser) fn begin_artifact(
        &mut self,
        kind: ArtifactKind,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        let (arguments, opened) = phase_header(arguments, kind.keyword(), line, source)?;
        if self.opened_block.is_some() {
            return Err(ParseError::new(
                line,
                source,
                "phase blocks are single-level; close the current block with } first",
            ));
        }
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
        if opened {
            self.opened_block = Some(super::super::machine::OpenedBlock {
                kind: kind.keyword(),
                name: name.to_owned(),
                line,
            });
        }
        Ok(())
    }

    pub(in crate::parser) fn start(
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

    pub(in crate::parser) fn env(
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

    pub(in crate::parser) fn secret(
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

    pub(in crate::parser) fn port(
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

    pub(in crate::parser) fn listener(
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

    pub(in crate::parser) fn health_probe(
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
            [target, marker, duration] if target.starts_with('/') => {
                let name = self.current_artifact_name(directive, line, source)?;
                let service = &self.artifacts[name].service;
                (path_probe(service, target, directive, line, source)?, *marker, *duration)
            }
            [target, marker, duration] => {
                (probe_url(target, line, source)?, *marker, *duration)
            }
            [kind @ ("http" | "tcp"), target, _, _] => {
                return Err(ParseError::new(
                    line,
                    source,
                    format!(
                        "{directive} no longer uses `{kind} <target>`; write `{directive} {kind}://{target} {parameter} <duration>`"
                    ),
                ));
            }
            _ => {
                return Err(ParseError::new(
                    line,
                    source,
                    format!(
                        "{directive} requires `<http://host/path|tcp://host:port|/path|notify> {parameter} <duration>`"
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

    pub(in crate::parser) fn directory(
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

    pub(in crate::parser) fn claim(
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

    pub(in crate::parser) fn shm(
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

    pub(in crate::parser) fn stop_signal(
        &mut self,
        line: usize,
        source: &str,
        arguments: &str,
    ) -> Result<(), ParseError> {
        self.require_artifact_kind(
            "STOPSIGNAL",
            line,
            source,
            &[ArtifactKind::Service, ArtifactKind::App],
        )?;
        let signal = exact_fields(arguments, 1, line, source, "STOPSIGNAL <signal>")?[0];
        if !known_signal(signal) {
            return Err(ParseError::new(
                line,
                source,
                format!("STOPSIGNAL requires a known signal name, got {signal:?}"),
            ));
        }
        let service = self.current_service_mut("STOPSIGNAL", line, source)?;
        if service.stop_signal.replace(signal.to_owned()).is_some() {
            return Err(ParseError::new(
                line,
                source,
                "STOPSIGNAL is already declared for this artifact",
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
}
fn known_signal(signal: &str) -> bool {
    matches!(
        signal,
        "SIGHUP"
            | "SIGINT"
            | "SIGQUIT"
            | "SIGILL"
            | "SIGTRAP"
            | "SIGABRT"
            | "SIGBUS"
            | "SIGFPE"
            | "SIGKILL"
            | "SIGUSR1"
            | "SIGSEGV"
            | "SIGUSR2"
            | "SIGPIPE"
            | "SIGALRM"
            | "SIGTERM"
            | "SIGSTKFLT"
            | "SIGCHLD"
            | "SIGCONT"
            | "SIGSTOP"
            | "SIGTSTP"
            | "SIGTTIN"
            | "SIGTTOU"
            | "SIGURG"
            | "SIGXCPU"
            | "SIGXFSZ"
            | "SIGVTALRM"
            | "SIGPROF"
            | "SIGWINCH"
            | "SIGIO"
            | "SIGPWR"
            | "SIGSYS"
    )
}
