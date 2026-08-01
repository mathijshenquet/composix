# The dev loop: watch without lying

Status: proposal, 2026-08-01. Decision pending. Ledger row: compose
`watch` "❓ interesting dev loop, unscoped".

## 1. The problem

The flask-redis corpus row is the archetype: a compose file that bind-
mounts the source tree into the running container so the framework's
auto-reload picks up edits. We refuse that deploy-side (the running
artifact stops being the built artifact — the volume-masking idiom
docs/corpus.md row 4 rejects), but the *need* is legitimate: edit →
see it running, fast. What is composix's honest dev loop, and does it
deserve CLI surface?

## 2. Prior work

**Docker compose `watch`** (the modern replacement for the source-bind
idiom) declares per-path actions: `sync` (copy files into the running
container), `rebuild` (rebuild image + recreate container), `sync+restart`.
Notably: docker itself moved *away* from the bind-mount lie toward
explicit rebuild/sync actions. **Tilt/skaffold** industrialize the same
for k8s: watch sources, rebuild or live-sync, redeploy, stream status.
The `sync` family exists because docker rebuilds were slow; its cost is
that the running container diverges from any image that exists.

**The nix world** splits the loop: `nix develop` gives an inner loop on
bare metal (native tooling, hot reload, no artifact at all), and the
outer loop rebuilds honestly. **Composix already owns a fast outer
loop**: warm builder underlays (D71) put the measured warm edit at 7.5s
— 2× faster than crane's docker path (docs/nix-build.md) — and
`cix up`'s restart-changed already restarts exactly the services whose
items changed.

## 3. Recommendation

**Refuse `sync` forever** — copying edited files into a running service
recreates the bind-mount lie with extra steps. The composix dev loop is
**rebuild-and-restart, made automatic**: watch the Cixfile context,
warm-rebuild on change, restart-changed. With a 7.5s warm edit that is a
real inner loop, not a consolation prize — and the thing running is
always a real, addressable artifact.

Per D48(e) (build only at a real impedance mismatch), v0 is a
**documented recipe, not a feature**:
`watchexec -- cix up` (or `--restart` for the single-service case) in
docs/cixfile.md's workshop section, with ignore-guidance (workspace
dirs, `.git`). Ship the recipe with the wallos/D70 example, measure the
loop feel, and only then decide whether `cix watch` sugar (debounce,
context-derived ignores for free, unified output) earns existence.
Framework-native hot reload (flask debug, vite HMR) is explicitly the
inner-inner loop and belongs in `nix develop`, outside cix — document
the split rather than compete with it.

## 4. Open questions

1. Agree `sync` is a permanent ❌ (goes in docker.md as such), or keep
   it ❓ pending a corpus case where 7.5s is genuinely too slow (huge
   node_modules-class contexts)?
2. Recipe-first vs `cix watch` now: the recipe has zero maintenance but
   a real papercut (watchexec must ignore the workspace/lock churn cix
   itself causes — wrong ignores = infinite rebuild loop). Is that
   papercut alone enough to justify the subcommand early?
3. Does `cix up` need a `--watch`-adjacent quality first: partial
   restarts already work, but build output during iteration is noisy —
   worth a quiet mode in the same round?
