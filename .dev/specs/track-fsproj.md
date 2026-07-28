# Track: fsproj — filesystem projection (D22 v3) + stress tests

Read `docs/design.md` D22 (v3) — it is the contract, written after a live systemd probe on
this host confirmed feasibility. This replaces the `/app` stable mount entirely. Where
ambiguous, choose boring and note it in LOG. Do NOT expand scope.

## Ground rules

- Work log: append `crates/cix-run/LOG.md`. Territory: `crates/cix-run/`,
  `crates/cix-cixfile/`, `examples/`, `docs/design.md` (ONLY the Part 4 interpolation-rule
  sentence that still mentions `/app`), `docs/cixfile.md` (worked example + prose that mention
  `/app`), `docs/tour/` via the generator only.
- Sudo available; clean up all units, always. COMMIT AS YOU GO; clean `git status --short`.

## Deliverables

1. **Spec schema**: new `"mounts": ["/abs/prefix", …]` per service (cixSpec 2 — additive,
   deny_unknown means old runners reject items using it, which is correct). Validation:
   absolute, normalized (no `..`, no trailing `/`), no duplicates/nesting among mounts, no
   overlap with any declared role dir path, deny-list per D22 v3 (`/nix`, `/proc`, `/sys`,
   `/dev`, `/run`, `/var/lib`, `/var/cache`, `/var/log` as *exact* roots — one component
   below them is role territory; plus exact paths `/etc/passwd`, `/etc/group`,
   `/etc/nsswitch.conf`, `/`, `/etc` itself, `/usr`, `/bin`, `/lib*`).
2. **Runtime**: each mount → `BindReadOnlyPaths=<store-item><mount>:<mount>` in system mode;
   validate the item actually contains the source path (clear error otherwise). Remove the
   `/app` bind and its env var from system mode. Degraded `--user`: warn that mounts cannot be
   projected, keep exposing the store path as `CIX_APP` (per D22 v3).
3. **cix-cixfile**: `FILE`/`SCRIPT`/`COPY`/`LINK` destinations may be absolute (projected) or
   bare-relative (item-internal, exec-target zone). Compiler computes the minimal mount set:
   group absolute destinations by their top-most dedicated prefix (e.g. `/etc/nginx/*` →
   `/etc/nginx`; never emit a mount broader than a declared destination's first two
   components; a root-level single file is its own mount). Emit into the generated spec.
   Parser errors for destinations hitting the deny-list, citing D22 v3.
4. **Examples migrated** (both Cixfile and default.nix variants): nginx config at
   `/etc/nginx/nginx.conf`, content at `/srv/www`, mime.types linked at
   `/etc/nginx/mime.types`; postgres scripts stay item-internal (`bin/…`), no projection
   needed unless natural. Both demos re-verified under sudo, both build paths (cix build +
   nix-build).
5. **Stress tests** (this is explicitly a stress-testing track — adversarial unit +
   integration tests, real systemd where applicable):
   - collision: mount overlapping a role dir → validation error (both orders);
   - deny-list: each denied root/file → error citing the rule;
   - nesting: mounts `/etc/nginx` + `/etc/nginx/conf.d` → error (nested);
   - single-file mount at root level (`/cix-probe.conf`) → works;
   - deep chain (`/opt/a/b/c/d`) → works;
   - shadowing an EXISTING host dir (e.g. `/etc/ssl`) → document observed in-unit behavior
     (host masked, read-only) in a test + LOG note;
   - symlink escape: item content containing an absolute symlink to `/etc/shadow` — assert
     the unit cannot read through it to secrets (read-only + normal perms apply), document;
   - volume: 25 mounts on one service → unit starts, all visible;
   - missing source in item → clear build/run error.
6. **Docs**: update the two `/app` mentions in design.md Part 4 + docs/cixfile.md (worked
   example matches the migrated `examples/nginx/Cixfile` exactly, verbatim note updated);
   regenerate the tour if any transcript mentions `/app` or the degraded warning text changed.

## Done gate

fmt/clippy/tests green ×2; both sudo demos green on both build paths; the VM check
(`nix build .#checks.x86_64-linux.vm-dogfood`) green; no leftover units; committed; LOG final
summary with every stress-test observation.
