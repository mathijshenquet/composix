# Track: ocimport — PROTOTYPE a docker/OCI compatibility layer (`cix import`)

Goal: answer the docker.md open question "No OCI import path — worth a track, or a
distraction?" with a working prototype and an honest report. This is an EXPERIMENT: learning
is the deliverable, merging is optional. Composix will never RUN OCI images natively
(docs/docker.md "Scope, stated once"); the bet is that IMPORTING one into a store item + a
generated spec is cheap enough to soften the migration cliff.

Read first: `docs/design.md` (D8, D11, D20–D22 v3), `docs/docker.md` "Scope" + section 1.

## Ground rules

- Work log: create/append `crates/cix-import/LOG.md` — walls and findings matter more than
  polish. Territory: new crate `crates/cix-import/` + CLI wiring in `crates/cix/src/main.rs`
  + workspace Cargo.toml. Nothing else (no changes to cix-run/cix-cixfile semantics; if the
  import needs runtime features that don't exist, RECORD them as findings instead).
- Sudo available for run experiments; clean up all units. COMMIT AS YOU GO.

## Scope

1. **Input**: a `docker-archive` tarball (`docker save` output) or OCI layout directory —
   offline, no registry auth needed. (`skopeo` is available via nix if you need format
   conversion; shelling out is fine for a prototype.) Do NOT build a registry client.
2. **Unpack**: apply layers in order with whiteout handling → a rootfs; `nix store add-path`
   the result (deterministic enough for a prototype; note determinism caveats in LOG).
3. **Generate a spec** from the image config: `Entrypoint`+`Cmd` → `exec` (rootfs-relative),
   `Env` → env defaults, `ExposedPorts` → ports (value form), `Volumes` → `dirs.state`
   entries, `WorkingDir`/`User` → record as LOG findings where the spec cannot express them.
4. **Run experiments**: the imported item is a FULL rootfs — our normal projection model does
   not apply. Experiment with `RootDirectory=<store-item>/rootfs` plus as much of the
   standard hardening as survives; document exactly which hardening had to be dropped and
   why. Targets, in order: docker library `nginx` image serving a page; `redis` answering
   PING. Use real images (pull tarballs once with skopeo; you have network).
5. **Report** (final LOG entry, the actual deliverable): what works, what breaks, the
   irreducible differences (uid assumptions, writable-rootfs expectations, entrypoint scripts
   doing apt-style mutations), estimated effort to productionize, and your verdict:
   worth-a-track / distraction, with reasoning. If the prototype is clean enough to merge as
   an explicitly-experimental subcommand (`cix import`, marked experimental in help text),
   say so; otherwise the branch stays unmerged and the report is the value.

## Done gate

Prototype demonstrably imports and runs at least the nginx image end-to-end (or documents
precisely why impossible); fmt/clippy/tests green for the new crate; committed; the report
written.
