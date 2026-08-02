use std::{
    fmt::Write as _,
    process::{Command, Output},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct ListedUnit {
    unit: String,
    active: String,
    sub: String,
    description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PsRow {
    pub manager: String,
    pub composite: String,
    pub service: String,
    pub unit: String,
    pub state: String,
    pub result: String,
    pub description: String,
}

pub fn ps(json: bool) -> Result<()> {
    let rows = rows()?;
    if json {
        println!("{}", render_ps_json(&rows)?);
    } else {
        println!("{}", render_ps_table(&rows));
    }
    Ok(())
}

fn rows() -> Result<Vec<PsRow>> {
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
                    rows.push(PsRow {
                        manager: manager.into(),
                        composite,
                        service,
                        unit: unit.unit,
                        state: format!("{}/{}", unit.active, unit.sub),
                        result,
                        description: unit.description,
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
    rows.sort_by(|left, right| {
        left.manager
            .cmp(&right.manager)
            .then_with(|| left.composite.cmp(&right.composite))
            .then_with(|| left.service.cmp(&right.service))
            .then_with(|| left.unit.cmp(&right.unit))
    });
    Ok(rows)
}

pub fn render_ps_json(rows: &[PsRow]) -> Result<String> {
    serde_json::to_string_pretty(rows).context("serializing cix ps JSON")
}

pub fn render_ps_table(rows: &[PsRow]) -> String {
    let manager_width = width(rows, 7, |row| row.manager.len());
    let composite_width = width(rows, 9, |row| row.composite.len());
    let service_width = width(rows, 7, |row| row.service.len());
    let unit_width = width(rows, 4, |row| row.unit.len());
    let result_width = width(rows, 6, |row| row.result.len());
    let mut table = format!(
        "{:<manager_width$}  {:<composite_width$}  {:<service_width$}  {:<unit_width$}  {:<10}  {:<result_width$}  DESCRIPTION",
        "MANAGER", "COMPOSITE", "SERVICE", "UNIT", "STATE", "RESULT"
    );
    for row in rows {
        write!(
            table,
            "\n{:<manager_width$}  {:<composite_width$}  {:<service_width$}  {:<unit_width$}  {:<10}  {:<result_width$}  {}",
            row.manager,
            row.composite,
            row.service,
            row.unit,
            row.state,
            row.result,
            row.description,
        )
        .expect("writing cix ps table row");
    }
    table
}

fn width(rows: &[PsRow], minimum: usize, value: impl Fn(&PsRow) -> usize) -> usize {
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
    if let Ok(fields) = systemctl_value(user, &service_unit, "LogExtraFields") {
        let field = |name: &str| {
            fields
                .split_whitespace()
                .find_map(|entry| entry.strip_prefix(&format!("{name}=")))
                .map(str::to_owned)
        };
        if let (Some(composite), Some(service)) = (field("CIX_COMPOSITE"), field("CIX_SERVICE")) {
            return Ok((composite, service));
        }
    }
    let slice = systemctl_value(user, &service_unit, "Slice")?;
    let Some(composite) = slice
        .strip_prefix("cix-")
        .and_then(|value| value.strip_suffix(".slice"))
    else {
        return Ok(("-".into(), service_label("-", &service_unit)));
    };
    if composite == "run" {
        return Ok(("-".into(), run_service_label(&service_unit)));
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

fn run_service_label(unit: &str) -> String {
    let label = service_label("run", unit);
    label
        .rsplit_once('-')
        .filter(|(_, nonce)| {
            nonce.len() == 24 && nonce.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .map(|(service, _)| service.to_owned())
        .unwrap_or(label)
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
    use super::{render_ps_json, render_ps_table, run_service_label, service_label, PsRow};

    #[test]
    fn labels_compose_services_and_edges() {
        assert_eq!(service_label("stack", "cix-stack-web.service"), "web");
        assert_eq!(
            service_label("stack", "cix-stack-edge-database.service"),
            "edge/database"
        );
        assert_eq!(
            run_service_label("cix-run-tour-app-0123456789abcdeffedcba98.service"),
            "tour-app"
        );
    }

    #[test]
    fn json_output_is_a_stable_golden_view_of_table_rows() {
        let rows = vec![PsRow {
            manager: "user".into(),
            composite: "stack".into(),
            service: "web".into(),
            unit: "cix-stack-web.service".into(),
            state: "active/running".into(),
            result: "success".into(),
            description: "stack/web".into(),
        }];
        assert_eq!(
            render_ps_json(&rows).unwrap(),
            r#"[
  {
    "manager": "user",
    "composite": "stack",
    "service": "web",
    "unit": "cix-stack-web.service",
    "state": "active/running",
    "result": "success",
    "description": "stack/web"
  }
]"#
        );
        assert_eq!(
            render_ps_table(&rows),
            "MANAGER  COMPOSITE  SERVICE  UNIT                   STATE       RESULT   DESCRIPTION\nuser     stack      web      cix-stack-web.service  active/running  success  stack/web"
        );
    }
}
