use super::super::*;

pub(crate) fn chapter_runtime_contract() -> String {
    let mut doc = Doc::new("runtime-contract");
    fs::write(
        doc.base.join("server.py"),
        r#"#!/usr/bin/env python3
import os
from pathlib import Path
from http.server import BaseHTTPRequestHandler, HTTPServer

def state_root():
    native = Path(os.environ["STATE_DIRECTORY"].split(":")[0])
    cache = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state"))
    fallback = cache / "cix-run-web/var/lib/runtime-guide"
    for candidate in (native, fallback):
        try:
            candidate.mkdir(parents=True, exist_ok=True)
            probe = candidate / ".write-probe"
            probe.write_text("ok")
            probe.unlink()
            return candidate
        except PermissionError:
            continue
    raise RuntimeError("no writable managed state directory")

state_file = state_root() / "value"
liveness_ok = True

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        global liveness_ok
        if self.path == "/state":
            body = state_file.read_bytes() if state_file.exists() else b"empty\n"
            status = 200
        elif self.path == "/fail-live":
            liveness_ok = False
            body = b"liveness will fail\n"
            status = 200
        elif self.path == "/livez" and not liveness_ok:
            body = b"unhealthy\n"
            status = 503
        else:
            body = b"runtime healthy\n"
            status = 200
        self.send_response(status)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_PUT(self):
        length = int(self.headers.get("Content-Length", "0"))
        state_file.write_bytes(self.rfile.read(length) + b"\n")
        body = b"stored\n"
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):
        pass

print("runtime service started", flush=True)
HTTPServer(("127.0.0.1", 8420), Handler).serve_forever()
"#,
    )
    .expect("writing runtime HTTP server");
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(doc.base.join("server.py"))
            .expect("reading runtime server permissions")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(doc.base.join("server.py"), permissions)
            .expect("making runtime server executable");
    }
    fs::write(
        doc.base.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

SERVICE web
IMPORT ${pkgs.coreutils} ${pkgs.python3}
COPY server.py /bin/runtime-server
START runtime-server
PORT http = 8420
STATEDIR /var/lib/runtime-guide
SECRET db-password AS DB_PASSWORD_FILE
READINESS http://127.0.0.1:8420/healthz IN 10s
LIVENESS http://127.0.0.1:8420/livez EVERY 2s

APP cleanup
IMPORT ${pkgs.coreutils}
START true

SERVICE observer
IMPORT ${pkgs.bash} ${pkgs.coreutils}
COPY observer.sh /bin/runtime-observer
START runtime-observer
"#,
    )
    .expect("writing runtime Cixfile");
    fs::write(
        doc.base.join("observer.sh"),
        "#!/bin/bash\nset -eu\nprintf '%s\\n' 'observer ready'\nexec sleep 300\n",
    )
    .expect("writing runtime observer");
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(doc.base.join("observer.sh"))
            .expect("reading runtime observer permissions")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(doc.base.join("observer.sh"), permissions)
            .expect("making runtime observer executable");
    }
    fs::write(doc.base.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing runtime lock");

    doc.para("You will run an HTTP service twice, observe readiness, preserve state across the restart, inspect its systemd unit, validate the real credential-supply document, and schedule a finite command. The rootless receipts exercise process, port, health, managed state, timer, and observability behavior; production-only secret delivery and sealed filesystem isolation are labelled where they require the root system manager.");

    doc.para("## The item owns needs; the operator owns values");
    doc.para("The web item declares the process needs: a direct TCP port, application-native persistent state, one credential filename, and HTTP health checks. `READINESS http://127.0.0.1:8420/healthz IN 10s` means the native cix probe tries localhost every 250 milliseconds, accepts an HTTP status from 200 through 399, and makes startup fail if none succeeds within ten seconds. `LIVENESS http://127.0.0.1:8420/livez EVERY 2s` probes every two seconds; three missed intervals trigger systemd's bounded `Restart=on-failure` policy. No curl or shell is added to the runtime item for those probes.");
    doc.para("The checked-in server uses `$STATE_DIRECTORY` at the native path when the manager can project it and the documented user backing below `~/.local/state/cix-run-web` otherwise. It does not treat `CIX_APP` as an application API: that variable exists only on the degraded user path to identify the physical store item, and is absent in the production system unit. The finite cleanup APP is eligible for scheduling; the minimal observer stays alive for scoped accounting receipts.");
    let source = ["Cixfile", "server.py", "observer.sh"]
        .map(|path| doc.show_file(path))
        .join("");
    assert!(source.contains("STATEDIR /var/lib/runtime-guide"));
    assert!(source.contains("COPY server.py /bin/runtime-server"));
    assert!(source.contains("START runtime-server"));
    assert!(source.contains("SECRET db-password AS DB_PASSWORD_FILE"));
    assert!(source.contains("READINESS http://127.0.0.1:8420/healthz IN 10s"));
    assert!(source.contains("LIVENESS http://127.0.0.1:8420/livez EVERY 2s"));
    assert!(source.contains("APP cleanup"));
    assert!(source.contains("SERVICE observer"));
    assert!(source.contains("START runtime-observer"));
    let built = doc.sh("cix build . --namespace runtime -t v1", true);
    assert!(built.contains("\"web\""));
    assert!(built.contains("\"cleanup\""));
    assert!(built.contains("\"observer\""));
    let runtime_refs = doc.sh("cix ls runtime/", true);
    assert!(runtime_refs.contains("runtime/web:v1"));
    assert!(runtime_refs.contains("runtime/cleanup:v1"));
    assert!(runtime_refs.contains("runtime/observer:v1"));

    doc.para("The build command's `--namespace runtime -t v1` creates the three refs printed above: `runtime/web:v1`, `runtime/cleanup:v1`, and `runtime/observer:v1`. `cix run` resolves one ref and compiles its manifest into a transient systemd service plus native health helpers.");

    doc.para("## Run, write state, restart, and read it");
    doc.para("This uses the degraded development path (decision D13): `--user` targets the per-user manager without `DynamicUser=` and may lose the sandbox controls listed in Chapter 1, but the process, readiness gate, port, and managed state still run. Host-varying degradation text is normalized to the declared marker.");
    let user_state_value = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .expect("HOME or XDG_STATE_HOME is set for the runtime state receipt")
        .join("cix-run-web/var/lib/runtime-guide/value");
    let prior_state_value = fs::read(&user_state_value).ok();
    let first_started = doc.sh(
        "unit=$(cix run runtime/web:v1 --user --detach); printf '%s\\n' \"$unit\"",
        true,
    );
    let first_web_unit = first_started
        .lines()
        .find(|line| line.starts_with("cix-run-web-") && line.ends_with(".service"))
        .expect("first web run printed its unit")
        .to_owned();
    wait_for_http(TOUR_LISTEN, "runtime healthy");
    let readiness = doc.sh("curl -fsS http://127.0.0.1:8420/healthz", true);
    assert_eq!(readiness.trim(), "runtime healthy");
    let written = doc.sh(
        "curl -fsS -X PUT --data 'kept across restart' http://127.0.0.1:8420/state",
        true,
    );
    assert_eq!(written.trim(), "stored");
    let inspected_runtime = doc.sh_with_env(
        "cix inspect --runtime --user \"$unit\" | jq '{unit, state, properties:{PrivateNetwork:.properties.PrivateNetwork, ProtectSystem:.properties.ProtectSystem, StateDirectory:.properties.StateDirectory}}'",
        &[("unit", &first_web_unit)],
        true,
    );
    assert!(inspected_runtime.contains("\"unit\""));
    assert!(inspected_runtime.contains("StateDirectory"));
    let health_properties = doc.sh_with_env(
        "systemctl --user show \"$unit\" -p TimeoutStartUSec -p WatchdogUSec -p Restart",
        &[("unit", &first_web_unit)],
        true,
    );
    assert!(health_properties.contains("Restart=on-failure"));
    assert!(health_properties.contains("WatchdogUSec=6s"));
    assert!(health_properties.contains("TimeoutStartUSec=10s"));
    doc.para("The readiness adapter is an `ExecStartPost` process run by cix: it retries the HTTP target and delays the service's active state until success. The liveness adapter is a second cix process that pings every two seconds and notifies systemd; the six-second watchdog shown above is the three-miss threshold, and systemd performs the restart.");
    let restarted = doc.sh_with_env(
        "pid_before=$(systemctl --user show \"$unit\" -p MainPID --value); curl -fsS http://127.0.0.1:8420/fail-live; pid_after=$pid_before; for attempt in $(seq 1 50); do pid_after=$(systemctl --user show \"$unit\" -p MainPID --value); if test \"$pid_after\" != \"$pid_before\" && test \"$pid_after\" != 0; then break; fi; sleep 0.25; done; test \"$pid_after\" != \"$pid_before\"; printf '%s\\n' 'liveness watchdog restarted the service'",
        &[("unit", &first_web_unit)],
        true,
    );
    assert!(restarted.contains("liveness watchdog restarted the service"));
    wait_for_http(TOUR_LISTEN, "runtime healthy");
    doc.sh_with_env(
        "systemctl --user stop \"$unit\"",
        &[("unit", &first_web_unit)],
        true,
    );
    wait_for_user_units_gone([first_web_unit.as_str()])
        .expect("first runtime web unit unloads after stop");
    stop_empty_cix_run_slice("the first runtime web run");

    let second_started = doc.sh(
        "unit=$(cix run runtime/web:v1 --user --detach); printf '%s\\n' \"$unit\"",
        true,
    );
    let second_web_unit = second_started
        .lines()
        .find(|line| line.starts_with("cix-run-web-") && line.ends_with(".service"))
        .expect("second web run printed its unit")
        .to_owned();
    wait_for_http(TOUR_LISTEN, "runtime healthy");
    let persisted = doc.sh("curl -fsS http://127.0.0.1:8420/state", true);
    assert_eq!(persisted.trim(), "kept across restart");
    doc.sh_with_env(
        "systemctl --user stop \"$unit\"",
        &[("unit", &second_web_unit)],
        true,
    );
    wait_for_user_units_gone([second_web_unit.as_str()])
        .expect("second runtime web unit unloads after stop");
    stop_empty_cix_run_slice("the second runtime web run");
    match prior_state_value {
        Some(contents) => fs::write(&user_state_value, contents)
            .expect("restoring pre-existing runtime state value"),
        None if user_state_value.exists() => {
            fs::remove_file(&user_state_value).expect("removing tour runtime state value")
        }
        None => {}
    }

    doc.para("`STATEDIR /var/lib/runtime-guide` is cix-owned durable data, not part of the item or a container layer. In this user demo its backing is `~/.local/state/cix-run-web/var/lib/runtime-guide`; production uses `/var/lib/cix-run-web/var/lib/runtime-guide`. Back it up as application data. A named compose deployment retains it across `down`; `cix down runtime-guide --purge --yes` is the explicit destructive purge.");

    doc.para("## Supply the declared secret");
    fs::write(
        doc.base.join("runtime-compose.json"),
        r#"{
  "cixCompose": 1,
  "name": "runtime-guide",
  "secrets": {
    "db-password": {"file": "/run/cix-runtime-guide-db-password"}
  },
  "children": {
    "web": {"item": "runtime/web:v1"}
  }
}
"#,
    )
    .expect("writing runtime secret compose fixture");
    doc.show_file("runtime-compose.json");
    doc.para("Direct `cix run` intentionally has no secret-value flag. The implemented supplying side is the top-level compose `secrets` map: the exact production setup is `sudo install -m 0600 runtime-secret /run/cix-runtime-guide-db-password` followed by `sudo env CIX_STATE_DIR=/var/lib/cix-index cix run --compose runtime-compose.json`. Systemd then uses `LoadCredential=db-password:/run/cix-runtime-guide-db-password`, mounts the root-owned file at `$CREDENTIALS_DIRECTORY/db-password`, and sets `DB_PASSWORD_FILE` to that path. The rootless harness can validate this document but cannot honestly activate its root-owned credential.");
    fs::write("/tmp/cix-tour-runtime-guide-db-password", "tour-secret\n")
        .expect("writing temporary tour credential validation source");
    fs::write(
        doc.base.join("runtime-compose-check.json"),
        fs::read_to_string(doc.base.join("runtime-compose.json"))
            .expect("reading runtime compose")
            .replace(
                "/run/cix-runtime-guide-db-password",
                "/tmp/cix-tour-runtime-guide-db-password",
            ),
    )
    .expect("writing rootless secret validation compose");
    let secret_check = doc.sh("cix compose check runtime-compose-check.json", true);
    assert_eq!(
        secret_check.trim(),
        "compose runtime-guide: 1 services, 0 edges, valid"
    );
    fs::remove_file("/tmp/cix-tour-runtime-guide-db-password")
        .expect("removing temporary tour credential validation source");

    doc.para("## Debug and observe");
    doc.para("`cix debug` resolves an item and replaces its normal START command in a fresh sandbox. The first `--user` is a cix option; the second bare `--` ends cix option parsing, so everything after it is the replacement argv. This receipt runs imported `printenv` to expose the sandbox's item-derived PATH; substitute any diagnostic command present in the item's imports.");
    let before_debug = user_cix_units().expect("listing user units before cix debug");
    let debugged = doc.sh("cix debug runtime/cleanup:v1 --user -- printenv PATH", true);
    assert!(debugged.contains("cix debug --user is degraded"));
    assert!(debugged.contains("/nix/store/"));
    stop_user_units_created_since(&before_debug, "cix-debug-cleanup-", "the cix debug receipt");
    stop_empty_cix_run_slice("the cix debug receipt");

    doc.para("The observer sibling prints one journal message and remains alive, so every receipt can select one tour-owned unit. `ps --json` selects that exact unit instead of formatting an ambient table whose widths depend on unrelated units. In `cix stats`, MANAGER says which systemd manager owns the unit, COMPOSITE is `run` for a unary invocation, and the SERVICE column contains that invocation's concrete transient unit name; the remaining columns are live accounting counters.");
    let started = doc.sh("cix run runtime/observer:v1 --user --detach", true);
    let observer_unit = started
        .lines()
        .find(|line| line.starts_with("cix-run-observer-") && line.ends_with(".service"))
        .expect("cix run printed an observer unit")
        .to_owned();
    let active = doc.run(
        &doc.state_dir,
        &format!("systemctl --user is-active {observer_unit}"),
        true,
    );
    assert_eq!(String::from_utf8_lossy(&active.stdout).trim(), "active");
    let ps = doc.sh(
        &format!(
            "cix ps --json | jq --arg unit '{observer_unit}' '.[] | select(.unit == $unit) | {{manager, service, unit, state}}'"
        ),
        true,
    );
    assert!(ps.contains("\"manager\": \"user\""));
    assert!(ps.contains("\"service\": \"observer\""));
    assert!(ps.contains(&format!("\"unit\": \"{observer_unit}\"")));
    assert!(ps.contains("\"state\": \"active/running\""));
    let stats = doc.sh_with_env(
        "cix stats 2>/dev/null | awk -v unit=\"$unit\" 'NR == 1 || $3 == unit'",
        &[("unit", &observer_unit)],
        true,
    );
    let mut stats_lines = stats.lines();
    assert_eq!(
        stats_lines.next(),
        Some("MANAGER  COMPOSITE  SERVICE  MEMORY  CPU  TASKS  IO  IP")
    );
    let stats_row = stats_lines
        .next()
        .expect("cix stats printed the observer row");
    assert!(
        stats_lines.next().is_none(),
        "unexpected cix stats rows: {stats}"
    );
    let stats_fields = stats_row.split_whitespace().collect::<Vec<_>>();
    assert!(
        stats_fields.len() >= 8,
        "unexpected cix stats row: {stats_row}"
    );
    assert_eq!(&stats_fields[..3], &["user", "run", observer_unit.as_str()]);

    let explained = doc.sh("cix logs run/observer --explain", true);
    assert!(explained.contains("journalctl CIX_COMPOSITE=run CIX_SERVICE=observer"));
    doc.sh("cix logs run/observer -n 20 >/dev/null 2>&1", true);
    doc.para("Unary `cix run` stamps `CIX_COMPOSITE=run` and `CIX_SERVICE=observer` into the unit's journal metadata, which is why `run/observer` is the exact log selector. `--explain` prints the equivalent journal fields without reading entries; omitting it, as in the preceding command, asks journald for the last 20 matching entries. Its host-formatted output is discarded here because some user-journal configurations do not retain custom fields; the production system-manager receipt is the authoritative log-content check.");
    stop_user_unit(&observer_unit, "the cix observability receipts");
    stop_empty_cix_run_slice("the cix observability receipts");

    doc.para("## The system-manager guarantees");
    doc.para("The normal system-manager path applies the declared hardening while retaining a conventional host root. `--closed-root` is a stricter, opt-in audit mode; try it on the secret-free observer with `sudo env CIX_STATE_DIR=/var/lib/cix-index cix run runtime/observer:v1 --closed-root --detach`. The sealed unit sees the Nix store, the item's read-only projections, generated identity and resolver files, declared role directories, and manager-projected sockets or credentials; an undeclared host path is absent. The rootless contract cannot guarantee that mount namespace, so the [closed-root audit scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/closedroot-audit.nix) executes the failed undeclared access and sealed-root inventory under the system manager.");
    doc.para("The [directory lifecycle scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/dirs2.nix), [secrets scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/secrets.nix), and [health scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/health.nix) execute production persistence, credential rotation, readiness blocking, and liveness restart without faking host privileges here.");

    doc.para("## Schedule the APP");
    doc.para("An APP runs to completion instead of staying active. `systemd-analyze calendar` validates and normalizes the same `OnCalendar` expression. `--schedule` creates a service/timer pair and arms it for the next matching time; it does not run the APP immediately and no polling daemon is involved.");
    let calendar = doc.sh(
        "systemd-analyze calendar '*-*-* 00:00:00' | sed -n '1p'",
        true,
    );
    assert_eq!(calendar.trim(), "Normalized form: *-*-* 00:00:00");
    let scheduled = doc.sh(
        "timer=$(cix run runtime/cleanup:v1 --user --schedule '*-*-* 00:00:00'); printf '%s\\n' \"$timer\"",
        true,
    );
    let timer = scheduled
        .lines()
        .find(|line| line.starts_with("cix-run-cleanup-") && line.ends_with(".timer"))
        .expect("scheduled APP printed its timer")
        .to_owned();
    let shown = doc.sh_with_env(
        "systemctl --user show \"$timer\" -p Id -p ActiveState -p Unit",
        &[("timer", &timer)],
        true,
    );
    assert!(shown.contains("ActiveState=active"));
    let removed = doc.sh_with_env(
        "systemctl --user stop \"$timer\"; stem=${timer%.timer}; rm -f \"$XDG_RUNTIME_DIR/systemd/user/$stem.timer\" \"$XDG_RUNTIME_DIR/systemd/user/$stem.service\" \"$XDG_RUNTIME_DIR/systemd/user/$stem-root.service\"; systemctl --user daemon-reload; systemctl --user is-active \"$timer\"",
        &[("timer", &timer)],
        false,
    );
    assert_eq!(removed.trim(), "inactive");
    let service = format!("{}.service", timer.trim_end_matches(".timer"));
    let root_service = format!("{}-root.service", timer.trim_end_matches(".timer"));
    wait_for_user_units_gone([timer.as_str(), service.as_str(), root_service.as_str()])
        .expect("scheduled runtime units unload after explicit removal");
    stop_empty_cix_run_slice("the scheduled APP receipt");

    doc.para("You now have the complete ownership split: artifacts declare their process needs, compose supplies host policy and secrets, and systemd owns lifecycle, health, logs, timers, and accounting.");
    doc.finish()
}
