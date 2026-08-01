use std::process::{Command, Output};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ListedUnit {
    unit: String,
    active: String,
    sub: String,
    description: String,
}

struct Row {
    manager: &'static str,
    composite: String,
    service: String,
    unit: ListedUnit,
    result: String,
}

pub fn ps() -> Result<()> {
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    for (manager, user) in [("system", false), ("user", true)] {
        match list_units(user) {
            Ok(units) => {
                for unit in units {
                    let (composite, service) =
                        grouping(user, &unit.unit).unwrap_or_else(|_| ("-".into(), "-".into()));
                    let result = systemctl_value(user, &unit.unit, "Result")
                        .map(|result| crate::result_label(&result).to_owned())
                        .unwrap_or_else(|_| "-".into());
                    rows.push(Row {
                        manager,
                        composite,
                        service,
                        unit,
                        result,
                    });
                }
            }
            Err(error) => errors.push(format!("{manager}: {error:#}")),
        }
    }
    if rows.is_empty() && !errors.is_empty() {
        bail!("could not query systemd managers: {}", errors.join("; "));
    }
    for error in errors {
        eprintln!("warning: could not list {error}");
    }
    if !rows
        .iter()
        .any(|row| !matches!(row.composite.as_str(), "-" | "run"))
    {
        return cix_run::runtime::ps();
    }
    rows.sort_by(|left, right| {
        left.manager
            .cmp(right.manager)
            .then_with(|| left.composite.cmp(&right.composite))
            .then_with(|| left.service.cmp(&right.service))
            .then_with(|| left.unit.unit.cmp(&right.unit.unit))
    });

    let manager_width = width(&rows, 7, |row| row.manager.len());
    let composite_width = width(&rows, 9, |row| row.composite.len());
    let service_width = width(&rows, 7, |row| row.service.len());
    let unit_width = width(&rows, 4, |row| row.unit.unit.len());
    let result_width = width(&rows, 6, |row| row.result.len());
    println!(
        "{:<manager_width$}  {:<composite_width$}  {:<service_width$}  {:<unit_width$}  {:<10}  {:<result_width$}  DESCRIPTION",
        "MANAGER", "COMPOSITE", "SERVICE", "UNIT", "STATE", "RESULT"
    );
    for row in rows {
        println!(
            "{:<manager_width$}  {:<composite_width$}  {:<service_width$}  {:<unit_width$}  {:<10}  {:<result_width$}  {}",
            row.manager,
            row.composite,
            row.service,
            row.unit.unit,
            format!("{}/{}", row.unit.active, row.unit.sub),
            row.result,
            row.unit.description,
        );
    }
    Ok(())
}

fn width(rows: &[Row], minimum: usize, value: impl Fn(&Row) -> usize) -> usize {
    rows.iter().map(value).max().unwrap_or(minimum).max(minimum)
}

fn grouping(user: bool, unit: &str) -> Result<(String, String)> {
    if unit.ends_with(".slice") {
        return Ok((
            unit.strip_prefix("cix-")
                .and_then(|value| value.strip_suffix(".slice"))
                .unwrap_or("-")
                .to_owned(),
            "-".into(),
        ));
    }
    if unit.ends_with(".target") {
        return Ok((
            unit.strip_prefix("cix-")
                .and_then(|value| value.strip_suffix(".target"))
                .unwrap_or("-")
                .to_owned(),
            "-".into(),
        ));
    }
    let service_unit = if unit.ends_with(".socket") {
        systemctl_value(user, unit, "Service")?
    } else {
        unit.to_owned()
    };
    if !service_unit.ends_with(".service") {
        return Ok(("-".into(), "-".into()));
    }
    let slice = systemctl_value(user, &service_unit, "Slice")?;
    let Some(composite) = slice
        .strip_prefix("cix-")
        .and_then(|value| value.strip_suffix(".slice"))
    else {
        return Ok(("-".into(), service_label("-", &service_unit)));
    };
    if composite == "run" {
        return Ok(("-".into(), service_label("run", &service_unit)));
    }
    Ok((
        composite.to_owned(),
        service_label(composite, &service_unit),
    ))
}

fn service_label(composite: &str, unit: &str) -> String {
    let value = unit
        .strip_prefix(&format!("cix-{composite}-"))
        .and_then(|value| value.strip_suffix(".service"))
        .unwrap_or(unit);
    value
        .strip_prefix("edge-")
        .map(|edge| format!("edge/{edge}"))
        .unwrap_or_else(|| value.to_owned())
}

fn systemctl_value(user: bool, unit: &str, property: &str) -> Result<String> {
    let mut command = Command::new("systemctl");
    if user {
        command.arg("--user");
    }
    let output = command
        .args(["show", unit, "--property", property, "--value"])
        .output()
        .with_context(|| format!("querying {unit} property {property}"))?;
    if !output.status.success() {
        bail!("{}", command_error(&output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn list_units(user: bool) -> Result<Vec<ListedUnit>> {
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
        .context("invoking systemctl list-units")?;
    if !output.status.success() {
        bail!("{}", command_error(&output));
    }
    serde_json::from_slice(&output.stdout).context("systemctl emitted invalid JSON")
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
    use super::service_label;

    #[test]
    fn labels_compose_services_and_edges() {
        assert_eq!(service_label("stack", "cix-stack-web.service"), "web");
        assert_eq!(
            service_label("stack", "cix-stack-edge-database.service"),
            "edge/database"
        );
    }
}
