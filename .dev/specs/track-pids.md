# track/pids — PrivatePIDs=yes generator default (D36)

Read AGENTS.md at the repo root first. Authoritative design: docs/design.md **D36** (with
D13 for degraded fallback, D34 for the exec interplay). Where this spec and D36 disagree,
D36 wins.

## The change

1. The unit generator (crates/cix-run/src/unit.rs) emits `PrivatePIDs=yes` for every service
   in system mode, alongside the existing always-on hardening set.
2. Fallback: reuse the existing `namespace_fallback` degraded path — if systemd rejects the
   property (version < 257, user manager), drop it LOUDLY, same style as the current D13
   messages. Detect by the same mechanism the existing fallbacks use (start failure probing),
   not by version sniffing, unless the codebase already version-sniffs — follow local
   precedent.
3. `cix exec` needs no code change (it already joins whatever is private and reports it),
   but its live behavior changes: verify the banner now lists pid as private and the joined
   view shows only service processes with ns-local numbering.
4. Docs sweep: docker.md rows that currently say PID is host-shared (exec row, §9
   containment prose if it mentions it) must reflect the new default + the fallback honesty.
   D34's "PID is host-shared with today's generator" sentence: add "(superseded by D36)" —
   do not rewrite the decision record.
5. PID-1 caveat: one honest paragraph where the generator hardening is documented (zombie
   reaping / signal handlers now the app's duty; master/worker daemons unaffected).

## Verification gate (all required)

1. `cargo build`, `cargo test`, `cargo fmt --all --check`, `cargo clippy --workspace
   --all-targets -- -D warnings` clean. Golden/unit tests updated for the new property.
2. Live (sudo, run them): `cix run` nginx detached → service healthy; `cix exec nginx -- sh
   -c 'ls /proc'`-style check shows ONLY the service's processes (ns pid numbering, nginx
   master ≈ pid 1); banner lists pid under private namespaces. postgres example boots and
   serves (PID-1 reaping sanity). Stop and clean up.
3. `cix run --user` + tour: degraded path drops the property loudly; regenerate the tour if
   output changed; drift + determinism tests pass.
4. `nix build .#checks.x86_64-linux.vm-dogfood` passes (both examples under the new default).
5. If ANY example misbehaves as PID 1: do not work around it — record it in the LOG as
   spec-boundary evidence (D36) and report it; that is a successful outcome of this track.
6. Commit on branch track/pids. A finished task with no commit is a failed task.

## Log

Keep .dev/specs/track-pids.LOG.md current (append-only, timestamped), transcripts included.
