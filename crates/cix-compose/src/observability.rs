use std::{
    collections::BTreeMap,
    process::{Command, Output},
};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::unit_path;

#[derive(Clone, Debug, Default)]
pub struct LogsOptions {
    pub target: String,
    pub follow: bool,
    pub since: Option<String>,
    pub lines: Option<u32>,
    pub invocation: Option<String>,
    pub explain: bool,
}

pub fn logs(options: LogsOptions) -> Result<()> {
    let (composite, service) = split_target(&options.target)?;
    let mut arguments = Vec::new();
    if !options.explain {
        if let Some(namespace) = log_namespace(&composite, service.as_deref())? {
            arguments.push(format!("--namespace={namespace}"));
        }
    }
    if options.follow {
        arguments.push("-f".into());
    }
    if let Some(since) = options.since {
        arguments.extend(["--since".into(), since]);
    }
    if let Some(lines) = options.lines {
        arguments.extend(["-n".into(), lines.to_string()]);
    }
    arguments.push(format!("CIX_COMPOSITE={composite}"));
    if let Some(service) = service {
        arguments.push(format!("CIX_SERVICE={service}"));
    }
    if let Some(invocation) = options.invocation {
        arguments.push(format!("_SYSTEMD_INVOCATION_ID={invocation}"));
    }
    let equivalent = format!("journalctl {}", arguments.join(" "));
    if options.explain {
        println!("{equivalent}");
        return Ok(());
    }
    eprintln!("Equivalent: {equivalent}");
    let status = Command::new("journalctl")
        .args(&arguments)
        .status()
        .context("invoking journalctl")?;
    if status.success() {
        Ok(())
    } else {
        bail!("{equivalent} exited with {status}")
    }
}

pub fn stats() -> Result<()> {
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    for (manager, user) in [("system", false), ("user", true)] {
        match list_units(user) {
            Ok(units) => {
                for unit in units.into_iter().filter(|unit| unit.ends_with(".service")) {
                    match properties(user, &unit, PROPERTIES) {
                        Ok(values)
                            if values
                                .get("LogExtraFields")
                                .is_some_and(|fields| fields.contains("CIX_")) =>
                        {
                            rows.push(StatsRow::from_properties(manager, unit, values))
                        }
                        Ok(_) => {}
                        Err(error) => errors.push(format!("{manager} {unit}: {error:#}")),
                    }
                }
            }
            Err(error) => errors.push(format!("{manager}: {error:#}")),
        }
    }
    if rows.is_empty() && !errors.is_empty() {
        bail!("could not query cix accounting: {}", errors.join("; "));
    }
    for error in errors {
        eprintln!("warning: could not query {error}");
    }
    rows.sort_by(|left, right| {
        left.manager
            .cmp(right.manager)
            .then_with(|| left.unit.cmp(&right.unit))
    });
    println!("MANAGER  COMPOSITE  SERVICE  MEMORY  CPU  TASKS  IO  IP");
    for row in rows {
        println!(
            "{}  {}  {}  {}  {}  {}  {}  {}",
            row.manager, row.composite, row.service, row.memory, row.cpu, row.tasks, row.io, row.ip
        );
    }
    eprintln!("Live view: systemd-cgtop");
    Ok(())
}

pub fn result_label(result: &str) -> &str {
    if result == "watchdog" {
        "liveness watchdog missed"
    } else {
        result
    }
}

const PROPERTIES: &[&str] = &[
    "LogExtraFields",
    "MemoryCurrent",
    "CPUUsageNSec",
    "TasksCurrent",
    "IOReadBytes",
    "IOWriteBytes",
    "IPIngressBytes",
    "IPEgressBytes",
];

#[derive(Debug)]
struct StatsRow {
    manager: &'static str,
    unit: String,
    composite: String,
    service: String,
    memory: String,
    cpu: String,
    tasks: String,
    io: String,
    ip: String,
}

impl StatsRow {
    fn from_properties(
        manager: &'static str,
        unit: String,
        values: BTreeMap<String, String>,
    ) -> Self {
        let fields = values
            .get("LogExtraFields")
            .map(String::as_str)
            .unwrap_or_default();
        let field = |name| {
            fields
                .split_whitespace()
                .find_map(|entry| entry.strip_prefix(&format!("{name}=")))
                .unwrap_or("-")
                .to_owned()
        };
        let composite = field("CIX_COMPOSITE");
        let service = field("CIX_SERVICE");
        let (composite, service) = if composite == "-" {
            ("run".into(), field("CIX_RUN"))
        } else {
            (composite, service)
        };
        Self {
            manager,
            unit,
            composite,
            service,
            memory: accounting(&values, "MemoryCurrent"),
            cpu: accounting(&values, "CPUUsageNSec"),
            tasks: accounting(&values, "TasksCurrent"),
            io: pair(&values, "IOReadBytes", "IOWriteBytes"),
            ip: pair(&values, "IPIngressBytes", "IPEgressBytes"),
        }
    }
}

fn accounting(values: &BTreeMap<String, String>, name: &str) -> String {
    values
        .get(name)
        .filter(|value| available(value))
        .cloned()
        .unwrap_or_else(|| "-".into())
}

fn pair(values: &BTreeMap<String, String>, left: &str, right: &str) -> String {
    let left = accounting(values, left);
    let right = accounting(values, right);
    if left == "-" && right == "-" {
        "-".into()
    } else {
        format!("{left}/{right}")
    }
}

fn available(value: &str) -> bool {
    !value.is_empty() && value != "[not set]" && value != "18446744073709551615"
}

fn split_target(target: &str) -> Result<(String, Option<String>)> {
    let (composite, service) = match target.split_once('/') {
        Some((composite, service)) => (composite, Some(service)),
        None => (target, None),
    };
    if composite.is_empty() || service.is_some_and(|path| path.split('/').any(str::is_empty)) {
        bail!("logs target must be <composite>[/<child/path>]");
    }
    Ok((composite.to_owned(), service.map(str::to_owned)))
}

fn log_namespace(composite: &str, service: Option<&str>) -> Result<Option<String>> {
    let unit = match service {
        Some(service) => format!("cix-{composite}-{}.service", unit_path(service)),
        None => list_units(false)?
            .into_iter()
            .find(|unit| {
                unit.starts_with(&format!("cix-{composite}-")) && unit.ends_with(".service")
            })
            .unwrap_or_else(|| format!("cix-{composite}.target")),
    };
    Ok(properties(false, &unit, &["LogNamespace"])?
        .remove("LogNamespace")
        .filter(|namespace| !namespace.is_empty()))
}

fn properties(user: bool, unit: &str, names: &[&str]) -> Result<BTreeMap<String, String>> {
    let mut command = Command::new("systemctl");
    if user {
        command.arg("--user");
    }
    command.arg("show").arg(unit).arg("--no-pager");
    for name in names {
        command.arg(format!("--property={name}"));
    }
    let output = command
        .output()
        .with_context(|| format!("querying {unit}"))?;
    if !output.status.success() {
        bail!("{}", command_error(&output));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            line.split_once('=')
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
        })
        .collect())
}

fn list_units(user: bool) -> Result<Vec<String>> {
    let mut command = Command::new("systemctl");
    if user {
        command.arg("--user");
    }
    let output = command
        .args([
            "list-units",
            "cix-*",
            "--all",
            "--output=json",
            "--no-pager",
            "--no-legend",
        ])
        .output()
        .context("listing cix units")?;
    if !output.status.success() {
        bail!("{}", command_error(&output));
    }
    Ok(serde_json::from_slice::<Vec<ListedUnit>>(&output.stdout)?
        .into_iter()
        .map(|unit| unit.unit)
        .collect())
}

#[derive(Deserialize)]
struct ListedUnit {
    unit: String,
}

fn command_error(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stderr.trim().is_empty() {
        stdout.trim().to_owned()
    } else {
        stderr.trim().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_composite_or_member_log_targets() {
        assert_eq!(split_target("stack").unwrap(), ("stack".into(), None));
        assert_eq!(
            split_target("stack/api").unwrap(),
            ("stack".into(), Some("api".into()))
        );
        assert_eq!(
            split_target("stack/tier/api").unwrap(),
            ("stack".into(), Some("tier/api".into()))
        );
        assert!(split_target("stack//api").is_err());
    }

    #[test]
    fn watchdog_uses_the_health_vocabulary() {
        assert_eq!(result_label("watchdog"), "liveness watchdog missed");
    }

    #[test]
    fn accounting_off_is_a_dash() {
        let values = BTreeMap::from([("MemoryCurrent".into(), "18446744073709551615".into())]);
        assert_eq!(accounting(&values, "MemoryCurrent"), "-");
    }
}
