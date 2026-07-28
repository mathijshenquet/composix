# Track: vmtest — run the root dogfood loop in a disposable NixOS VM

Goal: the example demos (nginx, postgres) run as root inside a NixOS VM test instead of on
the developer's host — reproducible, CI-able, no host sudo. Use the NixOS test framework
(`pkgs.testers.runNixOSTest`), which boots a QEMU VM (KVM is available on this host) and
scripts it from python (`machine.succeed(...)`).

## Ground rules

- Work log: create/append `nix/LOG.md` (timestamped).
- Territory: `flake.nix`, `flake.lock`, `nix/` (new), and — only if strictly needed —
  `examples/*/default.nix` signatures (keep them callable standalone via
  `nix-build examples/<name>`; they already take `{ pkgs ? import <nixpkgs> {} }`).
  Do NOT touch `crates/` (another agent works there in parallel), `DESIGN.md`, `docs/`,
  `specs/`, `devenv.*`.
- COMMIT AS YOU GO. Done gate includes `git status --short` clean — an uncommitted worktree
  is a FAILED track.

## Deliverables

1. **`flake.nix`** (new; coexists with devenv): outputs
   - `packages.x86_64-linux.cix` — `rustPlatform.buildRustPackage` using the workspace
     `Cargo.lock` (`cargoLock.lockFile`), building the `cix` binary;
   - `checks.x86_64-linux.vm-dogfood` — the VM test below;
   - keep it minimal, pin nixpkgs via flake input, commit `flake.lock`.
2. **`nix/vm-dogfood.nix`** — a NixOS VM test:
   - node with the cix package in `environment.systemPackages` and nix available;
   - the example items are built on the HOST side as part of the test derivation (import
     `examples/nginx` and `examples/postgres` with the test's `pkgs`) and referenced by store
     path inside the VM (they land in the VM's store via the test closure automatically);
   - test script (as root in the VM): `cix run <nginx-path> --detach`, poll+`curl` the page,
     `cix ps` shows it, stop; then the same for postgres with `bin/psql` TCP `SELECT 1`;
     assert no `cix-*` units remain.
   - The VM has no network: confirm the demos need none (they must not `nix-build` inside the
     VM — everything comes pre-built via the closure).
3. **Runbook**: short `nix/README.md` — how to run (`nix flake check` /
   `nix build .#checks.x86_64-linux.vm-dogfood`), what it covers, typical runtime.
4. Run the check yourself to green at least twice.

## Notes

- The demo scripts (`examples/*/demo.sh`) are host-oriented (sudo, nix-build); do NOT reuse
  them inside the VM — script the equivalent steps directly in the test's python. Do not
  modify the demo scripts.
- If `buildRustPackage` hits a wall (e.g. workspace layout), solve it boringly and record it;
  do not restructure the workspace.

## Done gate

`nix build .#checks.x86_64-linux.vm-dogfood` green twice; flake evaluates on a clean
checkout; work committed; LOG final summary.
