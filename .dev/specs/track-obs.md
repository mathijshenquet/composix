# track/obs — CIP-83: observability projection

Read AGENTS.md first. Authoritative: docs/cips/0083-observability.md
(§3 + §5 Decision). Everything is a projection of journald/systemd
state — no cix storage, no daemon, every verb documents its raw
equivalent. NOTE: track/dirs runs concurrently and refactors
`add_directories` in crates/cix-run/src/unit.rs — your unit.rs change
is one additive property; keep it isolated so the merge seam stays
trivial. Work in `.worktrees/obs` on branch `track/obs`. Keep
`crates/cix-compose/LOG.md` current.

1. **Selector fields**: every generated unit gains
   `LogExtraFields=CIX_COMPOSITE=<comp> CIX_SERVICE=<svc>
   CIX_ITEM=<store path>` (compose) / `CIX_RUN=<unit> CIX_ITEM=…`
   (cix run). One property push, nothing else in unit.rs.
2. **`cix logs <comp>[/<svc>]`**: thin translation to `journalctl`
   over those fields; `-f`, `--since`, `-n`, `--invocation <id>` pass
   through. Default scope = unit lifetime. On first use (or with
   `--explain`) print the equivalent journalctl invocation.
3. **Exit causes**: `cix ps` gains a RESULT column and `cix inspect`
   runtime output gains result/exit fields, read from `systemctl show`
   (`Result=`, `ExecMainStatus=`, `ExecMainCode=`, `InvocationID=`).
   Map spawn exit codes 200–245 to their systemd.exec meanings (lookup
   table with short diagnoses); render result `watchdog` as
   "liveness watchdog missed" (CIP-79 wording, ready before the
   feature lands).
4. **`cix stats`**: one-shot table per composite member (and cix-run
   units) from accounting properties (`MemoryCurrent`, `CPUUsageNSec`,
   `TasksCurrent`, IO/IP when present, '-' when accounting is off).
   Live view = documented `systemd-cgtop` pointer, not wrapped.
5. **`logNamespace: true`** compose field (compose-ONLY per Decision —
   no cix run flag): members get `LogNamespace=cix-<comp>`; docs state
   the operational shift (`journalctl --namespace=cix-<comp>`, own
   retention config) and `cix logs` handles the namespace transparently
   when the field is set.
6. Docs: docs/docker.md rows for `logs`/`stats`/logging-drivers/
   per-app-retention updated honestly (✅/🔁 with the LogNamespace
   story); docs/compose docs for the new field; tour touch only if a
   page shows `cix ps` output (regen then).
7. Tests: unit-gen snapshots (fields present, namespace property);
   CLI-level: `cix logs --explain` prints the journalctl line; exit-code
   map unit tests; extend scenario-observability VM check: a member
   logs a marker line, `journalctl CIX_COMPOSITE=<comp>` finds it,
   `cix logs` finds it, `cix ps` shows a RESULT column, `cix stats`
   prints a row.

Gate: fmt / `cix fmt --check examples` / warning-denied clippy /
workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Exact repros in the LOG. Commit
on this branch when green.
