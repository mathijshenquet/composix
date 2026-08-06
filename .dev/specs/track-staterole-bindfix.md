# track/staterole-bindfix — arbitrary app-path state roles: the bind-hidden-by-readonly defect

Charter: corpus/migrate/docker/filebrowser/GAPS.md, the `→ language`
bullet: "the faithful system-manager run cannot write
`/config/settings.json`: realization mounts the arbitrary app-path
role root read-only after its managed bind, hiding the bind.
`STATEDIR /config` is the right lifecycle ... but it exhibits the same
arbitrary-path realization defect". Same class recorded 2026-08-06 for
filebrowser at corpus expansion.

Do:
1. **Characterize precisely**: locate the state-role realization for
   arbitrary app paths (unit generation/hardening in cix-run/cix-spec —
   the ReadOnlyPaths=/BindPaths=/TemporaryFileSystem= interplay).
   Reproduce the hidden-bind ordering concretely (unit properties in
   evidence, not inferred) before changing anything.
2. **Fix the realization ordering** so the managed bind at the role
   path stays visible and writable while the surrounding root keeps
   its intended read-only posture (systemd offers ReadWritePaths=/
   ordering to express this — pick the mechanism that keeps the
   DECLARED semantics unchanged; this is a realization repair, not a
   semantics change).
3. **Prove it**: a focused VM regression scenario for an
   arbitrary-path STATEDIR (write lands in the managed state dir,
   survives restart, root stays read-only elsewhere); filebrowser
   faithful run writes `/config/settings.json` and its `check.sh`
   `/health` probe passes; GAPS regraded honestly.
4. The GAPS bullet's second sentence (user-manager fallback cannot
   project declared app paths, then times out): characterize whether
   it is the same root cause; fix only if it is, otherwise record it
   as its own routed gap.

STOP-AND-REPORT if the repair turns out to require changing declared
semantics (what STATEDIR promises) rather than realization — that is
a joint decision, not this track's.

Gates: fmt, examples fmt, warning-denied clippy,
`cargo test --workspace`, tour regen+drift if touched, focused VM via
`devenv shell -- nix run .#progressive-vm-check`, corpus receipts for
filebrowser.

Discipline: branch `track/staterole-bindfix`, LOG `crates/cix/LOG.md`
(append, timestamped, FRICTION section). Synchronous value-checked
receipts only. Clean committed branch; do not merge.
