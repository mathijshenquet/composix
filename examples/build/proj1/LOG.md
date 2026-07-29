# Buildshape work log

- 2026-07-28 23:03 UTC — Started from a clean worktree and studied the reference repository
  read-only. Reduced the reusable shape to generic workspace topology, per-binary source
  filtering, shared dependency artifacts, and a separately locked frontend build. The stub
  will use only `common`, `core`, `api`, `worker`, `dashboard`, and `frontend`; no source
  names, dependencies, comments, or domain behavior will be reproduced. Next: build the
  minimal stub and pin its inputs.
- 2026-07-28 23:07 UTC — Completed the buildable stub with one shared Rust library, three
  binaries, and a dependency-free pnpm frontend. Removed the initially planned crate named
  `core` because that name collides with Rust's built-in crate during crane's dummy-source
  dependency build; `common` alone demonstrates the required shared internal dependency.
  Pinned nixpkgs, crane, rust-overlay, Rust 1.88.0, pnpm 11.17.0, and the frontend dependency
  store hash. All four package outputs built successfully: first successful output build
  2.60s (downloads were partly warm from hash calibration), identical warm build 0.39s.
  Smoke tests ran all binaries and checked both frontend files. A targeted filter test changed
  `worker/src/output.rs`: the worker derivation changed while the api derivation did not.
  Next: commit the stub, then write and evaluate both Cixfile BUILD variants.
- 2026-07-28 23:12 UTC — Wrote `docs/cixfile-build.md` with two complete Cixfiles over the
  stub. Variant A uses fixed Rust/pnpm builders and explicit stages; Variant B uses locked
  plugin items, typed Unix-like pipelines, and a canonical JSON planning protocol that
  cannot return raw Nix. Defined multi-output selection, SERVICE ownership, locks,
  diagnostics, expressiveness limits, and `.nix` graduation. Adversarial comparison
  recommends Variant A first; listed concrete evidence that could justify plugins later.
  Privacy deny-list scan across the example and document is clean. Next: commit the design,
  then run the final twice-build, content, scope, and privacy gates from committed state.
- 2026-07-28 23:13 UTC — Final done gate passed. From committed state, built the default and
  all four named package outputs twice (0.48s then 0.39s warm), and ran full `nix flake check`
  (0.28s). Workspace tests passed inside the pinned flake development environment. Verified
  each Rust output contains and runs only its named binary; verified the frontend contains
  exactly `index.html` and `main.js`. All changes since the track spec are confined to
  `examples/buildshape/` and `docs/cixfile-build.md`. Expanded privacy scan found no source
  identity, domain, product, dependency, path, or unique terms, and the read-only reference
  repository remains clean. Deliverables are complete in commits `7663745` and `fb64e10`;
  this entry is the final summary.
