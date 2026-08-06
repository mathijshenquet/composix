use super::super::*;

pub(crate) fn chapter_hello() -> String {
    let mut doc = Doc::new("hello");
    fs::write(
        doc.base.join("index.html"),
        "<h1>hello from your first composix service</h1>\n",
    )
    .expect("writing hello page");
    fs::write(
        doc.base.join("nginx.conf"),
        r#"daemon off;
error_log stderr info;
events { }
http {
  access_log off;
  client_body_temp_path /tmp/cix-tour-nginx-client-body;
  server { listen 8420; root srv/www; }
}
"#,
    )
    .expect("writing hello nginx config");
    fs::write(
        doc.base.join("start-hello"),
        r#"#!/usr/bin/env bash
set -eu
prefix=${CIX_APP:-/}
exec nginx -p "$prefix" -c etc/nginx/nginx.conf -e stderr -g 'pid /tmp/cix-tour-nginx.pid;'
"#,
    )
    .expect("writing hello launcher");
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(doc.base.join("start-hello"))
            .expect("reading hello launcher permissions")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(doc.base.join("start-hello"), permissions)
            .expect("making hello launcher executable");
    }
    fs::write(
        doc.base.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

SERVICE hello
IMPORT ${pkgs.nginx} ${pkgs.bash} ${pkgs.coreutils}
COPY index.html /srv/www/index.html
COPY nginx.conf /etc/nginx/nginx.conf
COPY start-hello /bin/start-hello
START start-hello
PORT http = 8420
CACHEDIR /var/cache/nginx
RUNDIR /run/nginx
"#,
    )
    .expect("writing hello Cixfile");
    fs::write(doc.base.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing hello lock");

    doc.para("You will build and run a small nginx service from ordinary checked-in files. A build result is an **item**: one immutable directory in `/nix/store` containing the program, its files, and a machine-readable service manifest. A Cixfile is the declaration that assembles that directory and states the process's runtime needs.");

    doc.para("## Before you start");
    doc.para("Install the current alpha with `nix profile install github:mathijshenquet/composix#cix`, or use `cix` from this repository's `devenv` shell. The commands below require Linux, Nix, and a per-user systemd manager; macOS, non-systemd Linux, and containers or WSL sessions without user systemd can follow the build sections but cannot run the service lifecycle.");
    let nix_version = doc.sh(
        "nix --version >/dev/null 2>&1 && printf 'nix: available\\n'",
        true,
    );
    assert!(nix_version.contains("nix: available"));
    let flakes = doc.sh(
        "nix flake metadata --no-write-lock-file github:NixOS/nixpkgs/624af665418d3c65d544145b4d34ad696439570e >/dev/null 2>&1 && printf 'flakes: available\\n'",
        true,
    );
    assert_eq!(flakes.trim(), "flakes: available");
    let manager = doc.sh(
        "case $(systemctl --user is-system-running 2>/dev/null) in running|degraded) printf 'user manager: available\\n';; esac",
        true,
    );
    assert!(manager.contains("user manager: available"));
    doc.para("Here **rootless** means that `cix run --user` asks your per-user systemd manager to start the unit without root privileges. This development path lacks `DynamicUser=` and may lose mount-namespace, device, PID, and capability restrictions that the system manager provides; cix prints that degradation instead of implying production-equivalent isolation.");

    doc.para("## Build the item");
    doc.para("`FROM … AS pkgs` selects a Nix package collection; the adjacent `Cixfile.lock` records its immutable Git revision and NAR hash, a fingerprint of the serialized source tree. `IMPORT` adds selected packages' command and data trees to the item, so `nginx` and `bash` can be named without host-installed copies. `${pkgs.nginx}` means the `nginx` package from the earlier `pkgs` name; `${…}` is Cixfile build-time substitution, not a shell variable.");
    doc.para("`SERVICE hello` declares a long-running process that systemd keeps active; Chapter 2 contrasts it with a finite APP and a non-runnable ITEM. The service copies its page, configuration, and launcher; names its entrypoint and inbound port; and declares two **role directories**, writable paths whose lifecycle systemd manages. `CACHEDIR` data may be cleaned and survives an ordinary restart, while `RUNDIR` is recreated for each service lifetime. In user mode their backing begins below `~/.cache/cix-run-hello` and `$XDG_RUNTIME_DIR/cix-run-hello`; the system manager uses `/var/cache/cix-run-hello` and `/run/cix-run-hello`. The launcher uses the real item path exposed as `CIX_APP` only on the degraded user path, so this demo keeps working when that manager cannot project the copied `/etc` and `/srv` paths.");
    let source = ["Cixfile", "index.html", "nginx.conf", "start-hello"]
        .map(|path| doc.show_file(path))
        .join("");
    assert!(source.contains("IMPORT ${pkgs.nginx}"));
    assert!(source.contains("START start-hello"));
    assert!(source.contains("CACHEDIR /var/cache/nginx"));
    assert!(source.contains("RUNDIR /run/nginx"));

    doc.para("Run from the directory containing these four files and `Cixfile.lock`. Capture a one-member build with a selector; this teaches the reusable shell idiom for every later command. The ellipsis in displayed output is normalization only: `$item` contains the complete path.");
    let built = doc.sh("item=$(cix build .#hello); printf '%s\\n' \"$item\"", true);
    let store_path = built
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("selected hello build printed an item")
        .to_owned();
    doc.para("`cix-manifest.json` is generated inside the item, not in the project. The build compiler derives its absolute command, read-only projections (mounts), port grant, and writable directory roles from the Cixfile; the runtime validates this manifest before compiling a unit.");
    let manifest = doc.sh_with_env(
        "jq '{start, mounts:[.mounts[] | select(. == \"/etc/nginx\" or . == \"/srv/www\")], ports, dirs}' \"$item/cix-manifest.json\"",
        &[("item", &store_path)],
        true,
    );
    assert!(manifest.contains("bin/start-hello"));
    assert!(manifest.contains("\"/var/cache/nginx\""));
    assert!(manifest.contains("\"/run/nginx\""));

    doc.para("## Run, probe, and stop it");
    doc.para("A **projection** is a read-only bind mount that makes an item path such as `$item/srv/www` appear at its declared service path such as `/srv/www`. The production system manager supplies those projections and stronger isolation; this rootless demo also has the `CIX_APP` fallback described above. Two displayed normalizations keep this page identical on every host: `NONCE` replaces the unique per-run identifier in unit names, and host-varying manager degradation warnings collapse to the fixed marker line `[manager degradation warnings vary by host — elided]`. The service, HTTP probe, and stop command still really execute.");
    let started = doc.sh_with_env(
        "unit=$(cix run \"$item\" --user --detach); printf '%s\\n' \"$unit\"",
        &[("item", &store_path)],
        true,
    );
    let unit = started
        .lines()
        .find(|line| line.starts_with("cix-run-hello-") && line.ends_with(".service"))
        .expect("hello run printed its unit")
        .to_owned();
    wait_for_http(
        TOUR_LISTEN,
        "<h1>hello from your first composix service</h1>",
    );
    let response = doc.sh("curl -fsS http://127.0.0.1:8420", true);
    assert_eq!(
        response.trim(),
        "<h1>hello from your first composix service</h1>"
    );
    doc.sh_with_env(
        &idempotent_user_stop_command("$unit"),
        &[("unit", &unit)],
        true,
    );
    wait_for_user_units_gone([unit.as_str()]).expect("hello unit unloads after stop");
    stop_empty_cix_run_slice("the Chapter 1 lifecycle");

    doc.para("You now have the complete first loop: checked-in files became one immutable item, its manifest became a named systemd unit, an HTTP request reached the real process, and the exact printed unit was stopped.");
    doc.finish()
}
