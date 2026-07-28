# Track: run (part 2) — cix-spec.json, unit generation, cix run

You are one of two parallel agents on this repo. Read `DESIGN.md` first — "Part 2 — spec + run"
plus the "Decisions so far" list is your contract (esp. D2, D8, D11, D13, D15). This file only
adds implementation constraints; where it and DESIGN.md conflict, DESIGN.md wins. Do not edit
DESIGN.md — propose changes as notes in your LOG instead.

## Ground rules

- Work log: keep `crates/cix-run/LOG.md` current (append-only, timestamped) per the global
  directives. Re-read it after any compaction.
- Territory: you own `crates/cix-run/` ONLY. `crates/cix-common/` belongs to the parallel index
  track — do NOT touch it; if you need a small helper (e.g. running `nix path-info`), duplicate
  it locally in cix-run and note it in LOG for post-merge dedupe. Do not touch
  `crates/cix-index/`, `crates/cix/src/main.rs` (already wired), or anything else.
- The CLI variants in `crates/cix-run/src/cli.rs` are the intended surface; adjust flags/help
  as needed, keep the enum shape.
- v0 accepts **store paths and flake installables** only; `name:tag` ref resolution belongs to
  the index track and gets wired after merge. `#service` suffix selects the service; optional
  when the spec has exactly one.
- Dependencies: add with judgment, prefer boring and light.
- Commit to your branch as you go, meaningful messages.

## Deliverables

1. **Spec types** for `$out/cix-spec.json` per the DESIGN.md schema: serde with
   `deny_unknown_fields` everywhere (D15), `cixSpec: 1` version check, typed env
   (`string|int|bool|port|path`, `default`/`required`/`secret`), named ports bound to env vars,
   dirs by role as absolute app paths (D11), `health` parsed but unused for now, optional
   `network: "host"`. Validation with friendly errors: exec paths resolve relative to `$out`,
   dir paths absolute and non-overlapping and outside `/nix`, port env vars must be declared,
   `$VAR` interpolation in exec only from declared env.
2. **Unit generation** as a pure function `(spec, service, resolved env/ports, mode) → unit
   file text`, per the DESIGN.md mapping table: `ExecStart` with absolute store paths and
   interpolated env; `Environment=` lines; `StateDirectory=`/`CacheDirectory=`/`LogsDirectory=`/
   `ConfigurationDirectory=` named `cix-run-<service>` (compose will change naming later) with
   `BindPaths=` mapping each host dir onto its declared app path; restrictive `*DirectoryMode=`;
   the full hardening block (see table — `DynamicUser`, `ProtectSystem=strict`, `PrivateTmp`,
   `NoNewPrivileges`, `SystemCallFilter=@system-service`, `CapabilityBoundingSet=`, address
   family logic: ports declared ⇒ allow AF_INET/AF_INET6, none and no `network: host` ⇒
   `PrivateNetwork=yes`, etc.). MemoryDenyWriteExecute default on; spec flag to opt out comes
   later — note it. **Golden-file tests**: fixture specs → expected unit text committed as
   fixtures.
3. **cix run**: validate env (`-e K=V` overrides, defaults, required, types), pick ports
   (`-p name=port` overrides port-typed env), then start a transient unit named
   `cix-run-<service>-<nonce>.service` via `systemd-run` (subprocess is fine for v0; D-Bus
   later). Default target: system manager (root; friendly error if not root). `--user` (D13):
   degraded dev mode against the user manager — no `DynamicUser`; attempt the mount-ns options
   (`BindPaths` etc.) with `PrivateUsers=yes` first, empirically verify what the user manager
   actually supports on this host (systemd on kernel 6.17), degrade gracefully with a loud
   warning listing what was dropped, and WRITE YOUR FINDINGS TO LOG — this is an open
   engineering question we want answered. Foreground: stream `journalctl -f -u` (or
   `--user-unit`), ctrl-C stops the unit; `--detach` prints the unit name and exits.
4. **cix ps**: list `cix-*` units from system + user managers (`systemctl list-units --output=json`),
   compact table.
5. **Test fixture**: build a spec'ed store item without nix builds: generate a dir containing
   `cix-spec.json` + a small shell script service (writes a timestamp into its state dir, then
   sleeps), `nix store add-path` it. Integration test (no root): `cix run --user` it, assert
   active, assert the state file appeared in the expected host-side dir, stop it cleanly. Golden
   tests cover system mode; root-only integration can live behind `#[ignore]`.
6. **demo.sh** in `crates/cix-run/`: fixture build + `cix run --user` + `cix ps` + stop,
   runnable by a human.

## Done criteria

`cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace` all
green; demo.sh works; LOG.md has a final summary entry listing deviations from DESIGN.md, the
--user-mode findings, and open questions for the maintainers.
