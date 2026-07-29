# track/inspect work log

## 2026-07-29T00:00:00Z

- Started D35(d,e) implementation on `track/inspect`. Read `AGENTS.md`, `.dev/LOG.md`, `docs/design.md` D35, and the track spec. Worktree is clean; the dev shell is not auto-activated, but `cargo` and `nix` are available from the configured profiles.
- Next: map the index and runtime APIs, implement the shared inspect surface and `ls -l` systems visibility, then run the required live/tour/VM gates.

## 2026-07-29T00:25:00Z

- Implemented the D35 inspect surface: `cix inspect` chooses artifact vs running-service worlds, defaults to stable JSON, has `--human`, and makes an artifact/service collision explicit through `--artifact` / `--runtime`. Artifact output preserves entry vocabulary and validates then serializes the manifest; runtime output uses the exact `cix exec` target-selection helper.
- `cix ls -l` is now a headed aligned table with a `SYSTEMS` column. Added focused dispatch/ambiguity/schema tests and regenerated the drift-checked tour with `12-inspecting.md`.
- Initial verification: `cargo check --workspace`, `cargo test -p cix --bin cix`, and `cargo test --test tour -- --ignored generate_tour` passed. Next: run the full gate and the required artifact + live nginx transcripts.

## 2026-07-29T00:40:00Z

- Live transcript (temporary `CIX_STATE_DIR`, then removed):

  ```text
  $ target/debug/cix build examples/pack/nginx --tag inspect-nginx:v1
  /nix/store/xw25mfnx5cdv9326hcp1k5mgfx5wk7f9-cixfile-item
  $ target/debug/cix inspect inspect-nginx:v1
  {"kind":"artifact", "reference":"inspect-nginx:v1", "storePath":"/nix/store/xw25mfnx5cdv9326hcp1k5mgfx5wk7f9-cixfile-item", "outputs":{"x86_64-linux":{…"drvPath":"/nix/store/59w5g52bpv39dsi3sg9032c5hid3k9bn-cixfile-item.drv"}}, "manifest":{"cixManifest":2,"services":{"nginx":…}}, "closureSize":53223304, "trustedKeys":[], "upstream":null, "drvPath":"/nix/store/59w5g52bpv39dsi3sg9032c5hid3k9bn-cixfile-item.drv"}
  $ sudo target/debug/cix run /nix/store/xw25mfnx5cdv9326hcp1k5mgfx5wk7f9-cixfile-item --detach
  warning: the system manager rejected PrivatePIDs isolation …
  warning: retrying without PrivatePIDs; this service shares the host PID namespace (D36 degraded fallback)
  cix-run-nginx-18c6cdcae80ada521.service
  $ sudo target/debug/cix inspect --runtime cix-run-nginx-18c6cdcae80ada521.service
  {"kind":"runtime", "unit":"cix-run-nginx-18c6cdcae80ada521.service", "service":"nginx", "state":{"load":"loaded","active":"active","sub":"running"}, "mainPid":3163057, "exitCause":{"result":"success","code":"0","status":"0"}, "ports":["tcp8080"], "listeners":{}, "dirs":{"cache":["/var/cache/private/cix-run-nginx"],"run":["/run/cix-run-nginx"]}, "properties":{…"DynamicUser":"yes", "PrivatePIDs":"no", "BindReadOnlyPaths":…, "SocketBindAllow":"tcp8080"…}}
  $ sudo systemctl stop cix-run-nginx-18c6cdcae80ada521.service
  ```

- Cleanup verified with `sudo systemctl list-units 'cix-run-nginx-*.service' --all --no-legend --plain` and `sudo systemctl list-unit-files 'cix-run-nginx-*.service' --no-legend`: both empty. Next: full Rust/tour/VM verification, final review, and commit.

## 2026-07-29T01:00:00Z

- Verification gate passed:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo test --test tour -- --ignored generate_tour` followed by `cargo test --test tour`
  - `nix build .#checks.x86_64-linux.vm-dogfood`
- The normal tour check caught age-cell drift after the new headed `ls -l` table; normalizing duration cells and regenerating the pages fixed it. Next: review the staged diff and commit on `track/inspect`.

## 2026-07-29T01:10:00Z

- Committed the completed track on `track/inspect` with message `feat: add cix inspect`.
