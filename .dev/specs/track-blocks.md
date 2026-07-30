# track/blocks — D47: the Cixfile becomes blocks and binders

Read AGENTS.md first. Authoritative design: docs/design.md **D47** (context: D31, D32,
D39, D40, D41). D47 wins on conflict. Scope: `crates/cix-cixfile` (grammar, model,
codegen), `crates/cix-run` (manifest `kind`, app run semantics), examples, docs, tour.
Do NOT touch `crates/cix-index`, `crates/cix-compose` semantics (compose consumes
manifests unchanged), or `nix/scenarios/**` (track/scenarios owns it, running
concurrently — expect to merge main before your final gate if it lands first).

## Grammar (hard rename, no aliases — same-day pre-release window)

1. **Blocks**: `BUILDER <name>`, `SERVICE <name>`, `APP <name>`, `ITEM <name>`.
   The prelude shrinks to `FROM` lines and top-level `FETCH` lines; everything else
   lives in a block. Block names share one namespace with FROM/FETCH binders;
   duplicates are line-numbered errors; references are backward-only.
2. **BUILDER**: allows COPY/FETCH/RUN/CACHE/PATH. `${<builder>}` = its final workdir
   snapshot (store path). D40 CACHE semantics unchanged but now per-builder (cache
   identity key gains the builder name); PATH here is the D31 build-time toolchain
   PATH, per-builder.
3. **SERVICE**: the D40/D41 artifact block as it exists today (COPY/FILE/SCRIPT/LINK/
   EXEC/SETUP/ENV/PORT/LISTENER…/OUTBOUND/dirs/health/jit), emitting a v4 bare
   manifest. **APP**: same assembly surface but run-to-completion semantics — allows
   EXEC/ENV/OUTBOUND/dirs(state,cache)/COPY/FILE/SCRIPT/LINK; REJECTS ports,
   listeners, health, jit, setup at parse (line-numbered, citing D47). Manifest gains
   `"kind": "app"`; absence of kind = service (v4 stays v4 — amend in place, it is
   hours old; migrate the golden fixtures). **ITEM**: assets only — COPY/FILE/LINK,
   no EXEC/ENV/ports; manifest `"kind": "item"` with no exec.
4. **RUN caged**: RUN outside a BUILDER is a parse error naming the doctrine ("RUN is
   only legal inside a BUILDER block"). **FETCH two forms**: in-builder chain step
   (unchanged D39 semantics), and top-level `FETCH <name> <command…>` — runs in an
   EMPTY workdir, network allowed, fixed-output TOFU-pinned in `Cixfile.lock` keyed by
   the name, memo key = command only. `${<fetch-name>}` = its snapshot store path.
5. **`FROM . AS <name>`**: binds the Cixfile's own directory as a source namespace.
   Not lock-pinned (it IS the input; content-addressing happens per-build via COPY
   snapshots, unchanged). Remote source FROMs (non-nixpkgs flakerefs) resolve + pin
   like any FROM input and expose their store path root for `${name}/…` COPY sources
   (attribute-path interpolation stays exclusive to package universes; a plain source
   input is a tree, `${src}/sub/path` only).
6. **COPY unification, TAKE dies**: a COPY source is either `${binder}/…` (or bare
   `${binder}` for whole-tree) or a bare relative path, which stays legal as the
   implicit Cixfile-directory context — docker's build context, unchanged from today
   (D47 amendment: adoption-bridge sugar). `FROM . AS <name>` is the optional explicit
   spelling of that same context; when declared, `${name}/…` and bare relative sources
   may coexist and mean the same root. Remote sources always need an explicit FROM
   binder. Destinations stay subject-relative (workdir in BUILDER, item root in
   artifact blocks) — absolute destinations remain invalid. In-artifact COPY from
   `${builder}/…` replaces TAKE 1:1; in-artifact COPY from a bare relative source
   copies from the Cixfile directory (assets straight into the item).
7. **`${build}` magic dies** with a migration-grade error message (unknown name
   `build` → "no binder named `build`; name your builder: `BUILDER build`").

## Runner (small)

- Parse/accept manifest `kind` (absent|"service"|"app"|"item"). `cix run` on kind=app:
  transient run-to-completion unit (Type=oneshot semantics, full hardening tier minus
  listener/port machinery), exit code propagated to the CLI. `cix run` on kind=item:
  refuse with a clear error. Timer/hook forms are OUT (D47 defers them).

## Migration & meta loops

- Migrate ALL example Cixfiles (pack/*, build/*, compose stack) to blocks+binders:
  add `FROM . AS src` where COPY sources exist, wrap chains in `BUILDER build`,
  ITEM→SERVICE renames, TAKE→COPY. proj1 keeps its two SERVICE items and gains
  nothing else. Add ONE new tiny example proving the new ingredient form:
  `examples/build/ingredient/` — top-level FETCH of a small pinned payload assembled
  into a SERVICE (keep the fetch tiny and stable; a nixpkgs-hosted tarball via
  ${pkgs.curl} is fine).
- Update the directive reference docs + docker.md rows that name SERVICE/ITEM/TAKE;
  add the Bazel positioning line D47 records. Regenerate the tour (expect real diffs:
  keyword renames, page 08/12/14 content — review honestly).
- proj1 gate must still pass end-to-end (selective rebuild, --no-cache byte-identity,
  tour run/curl) — it is the D40 acceptance and must survive the rename untouched in
  spirit.

## Gate (exact repro commands in crates/cix-cixfile/LOG.md)

`cargo fmt --all --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`cargo test --workspace` · tour regenerated + drift + determinism green ·
`nix build .#checks.x86_64-linux.vm-dogfood --no-link` ·
`nix build .#checks.x86_64-linux.compose-fallback-vm --no-link`.
