# Track: specv2 — implement cix-spec v2 in cix-run

Read `DESIGN.md` first: "Spec v2" under Part 2 is the contract (points 1–6 + principles
D20a/D20b), plus the existing Part 2 sections. This file adds implementation constraints.
Where ambiguous, choose the boring option and note it in your LOG — do NOT expand scope.

## Ground rules

- Work log: append to `crates/cix-run/LOG.md` (timestamped).
- Territory: `crates/cix-run/`, `examples/`. Nothing else.
- Passwordless sudo available for system-mode verification; ALWAYS clean up units
  (`systemctl list-units 'cix-*'` empty afterwards), never touch non-cix units.
- COMMIT AS YOU GO on this branch. The done gate includes `git log` showing your commits and
  `git status --short` clean — an uncommitted worktree is a FAILED track.

## Deliverables

1. **Version gating.** Accept `"cixSpec": 1` and `2`. One set of types is fine, but every
   v2-only field used under version 1 must produce a clear error naming the field and the
   required version. v1 semantics are unchanged.
2. **`dirs.run`** → `RuntimeDirectory=cix-run-<svc>[:alias]` with the same alias pattern as
   state/cache/logs. CAUTION, verify empirically: do NOT blindly add
   `TemporaryFileSystem=/run:ro` — the unit needs systemd's own `/run` plumbing (journal
   socket, etc.). Find the correct masking (or none) by testing a service with a run dir;
   record findings in LOG.
3. **`setup: [argv]`** → `ExecStartPre=` with the same `$out`-relative resolution and `$VAR`
   interpolation rules as `exec`. Document (rustdoc on the spec type) that it runs every
   start and must be idempotent.
4. **Fixed-value ports**: port entries are either `{"env": …}` or `{"value": N}` (exactly one;
   both/neither = validation error). `-p name=…` targeting a value port errors with
   "port is fixed at build time".
5. **Ports < 1024** (any env default, `-p` override, or value): add
   `AmbientCapabilities=CAP_NET_BIND_SERVICE` and set `CapabilityBoundingSet=
   CAP_NET_BIND_SERVICE` (instead of empty). Verify empirically with the caddy-style case:
   a value port 80 service actually binding 80 as DynamicUser (a tiny test service is fine;
   full caddy example comes in a later track).
6. **`jit: true`** → omit `MemoryDenyWriteExecute=yes`. Nothing else changes.
7. **D11 narrowing (validation)**: each role's app paths must be exactly one component under
   the role root (`/var/lib/<name>`, `/var/cache/<name>`, `/var/log/<name>`, `/etc/<name>`,
   `/run/<name>`). Deeper or foreign paths → validation error explaining the rule and citing
   DESIGN.md "Spec v2" point 6.
8. **Examples updated to v2**: postgres — `"cixSpec": 2`, socket dir → `dirs.run`
   `/run/postgresql` (`-k` flag accordingly), initdb moves out of `bin/start` into a `setup`
   script (start shrinks to nss_wrapper env + exec; setup needs the same nss_wrapper env —
   keep a small shared shell lib in the item). nginx — `"cixSpec": 2`, pidfile → `/run/nginx`,
   port becomes `{"value": 8080}`. Both demos re-verified end-to-end under sudo.
9. **Tests**: golden fixtures for the new mappings (run dir, setup, caps, jit, value ports);
   validation-error unit tests (v2-field-under-v1, both/neither port forms, path outside
   role root, -p on value port).

## Done gate

`cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`
green; both sudo demos pass; no leftover units; work committed (`git status --short` clean);
LOG.md final summary with walls + deviations.
