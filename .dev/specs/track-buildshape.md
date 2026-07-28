# Track: buildshape — stub the real-world flake shape + dream the Cixfile BUILD half

Goal: make the Cixfile BUILD story concrete against a REAL flake's shape. Source of truth for
the shape: `/home/mathijs/the private fleet repo` (READ-ONLY — study `flake.nix`, the `rust/` workspace
topology, `frontend/` build). PRIVACY RULE, absolute: this repo is public — the stub must
reproduce ONLY the shape (topology, build patterns, flake techniques), never details: no
crate/product/company names, no dependency lists, no business logic, no copied comments.
Generic names throughout (`api`, `worker`, `dashboard`, `frontend`, `core`, `common`).

## Ground rules

- Territory: `examples/buildshape/` (new) + `docs/cixfile-build.md` (new). Nothing else.
- Work log: `examples/buildshape/LOG.md`. COMMIT AS YOU GO; clean status at the end.
- Network available (crane/nixpkgs fetches).

## Deliverable 1: the stub (`examples/buildshape/`)

A minimal buildable reproduction of the shape:
- Cargo workspace (`rust/`): a few members incl. an internal lib crate two bins depend on
  (so source filtering and deps-only caching are MEANINGFUL), 2–3 binaries. Trivial hello-ish
  code; the topology is the point.
- Frontend (`frontend/`): a minimal pnpm project whose `pnpm build` emits `dist/` (keep deps
  near-zero; lockfile committed).
- `flake.nix` mirroring the real one's TECHNIQUES at ~1/3 the size: rust-overlay toolchain,
  crane, per-binary source filters, shared `buildDepsOnly` artifacts, `mkBin`-style helper,
  pnpm frontend package, multiple flake outputs. Must `nix build` green for every output;
  record build times cold/warm in LOG.

## Deliverable 2: the dream (`docs/cixfile-build.md`, exploratory design doc)

Design the Cixfile BUILD half against the stub, as TWO worked variants (full Cixfile text for
the stub in each), then judge them:

- **Variant A — inline minimal magic.** Docker-multistage-flavored directives with the least
  possible filtering/caching surface: think `STAGE deps` / `BUILD rust --bin api` /
  `COPY --from=stage`. Encode the crane techniques as a SMALL fixed set of knobs; state
  exactly which real-flake tricks (per-binary filters, vendor workarounds) fit and which are
  unexpressible (those fall to the `.nix` escape hatch).
- **Variant B — plugin system.** Assume flexible unix-tool-like plugins that encapsulate the
  cargo/crane knowledge: `USE cargo`, `USE pnpm`. Design the plugin CONTRACT precisely: what
  a plugin is (an executable/derivation implementing a defined protocol), what it receives
  (source, lockfiles, declared params), what it returns (a derivation / nix expr fragment),
  how plugins compose unix-style, how they're versioned and distributed (in cixpkgs? as store
  items themselves — plugins as items would be very composix), and what the Cixfile text
  looks like. The multistage/multi-output story: ONE Cixfile yielding MULTIPLE items
  (per-binary service items + a static frontend item) — design the output naming and how
  `cix build` selects (`cix build .#api`?), and how SERVICE blocks attach to specific
  outputs.
- Judge adversarially: expressiveness vs the real flake, magic budget (D20a spirit: no raw
  nix passthrough inside the Cixfile), lock/determinism story per variant, failure-mode
  legibility (what does a broken build's error look like), and the graduation path to `.nix`.
  End with a recommendation + what evidence would change it.

## Done gate

Stub builds green (`nix build` all outputs, twice); privacy rule audited (grep the stub +
doc for anything traceable to the source repo — names, unique strings); doc complete with
both full Cixfile renderings; committed; LOG final summary.
