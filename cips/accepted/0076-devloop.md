# The dev loop: watch without lying

Status: **CIP-76, adopted 2026-08-01** (Mathijs: "Lets ship noisy
first, signed off"). Ledger row was: compose `watch` "❓ interesting dev
loop, unscoped". Decision in §5.

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

**`cix watch` is a real subcommand** (review overruled the
recipe-first instinct: "seems like it will be popular, lets add it until
we remove it" — and the recipe has a real papercut: an external watcher
must hand-maintain ignores for the workspace/lock churn cix itself
causes, where cix derives them for free). Scope v0: watch the Cixfile
context with context-derived ignores, debounce, warm rebuild, restart
exactly the changed services — compose-wide, single-service being the
unary case. Framework-native hot reload (flask debug, vite HMR) is
explicitly the inner-inner loop and belongs in `nix develop`, outside
cix — document the split rather than compete with it.

## 4. Open questions

1. ~~sync~~ — killed in review, permanent ❌ in docker.md.
2. ~~recipe vs subcommand~~ — subcommand, per review.
3. ~~output ergonomics~~ — ship noisy first (adoption call).

## 5. Decision

`sync` is a permanent refusal (docker.md ❌: copying edits into a
running service recreates the bind-mount lie). `cix watch` is a real
subcommand: watch the Cixfile context with context-derived ignores,
debounce, warm rebuild, restart exactly the changed services;
compose-wide with single-service as the unary case. Ships noisy; quiet
mode when it annoys. Framework hot reload stays outside cix
(`nix develop`), documented as the split.

## Changelog

- 2026-08-01: drafted; amended after review — sync permanently refused,
  `cix watch` promoted from recipe to subcommand, quiet-mode question
  stays open.
