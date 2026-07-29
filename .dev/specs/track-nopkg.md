# Track: nopkg — scrap PKG, flake-shaped interpolation (D32)

Read `docs/design.md` D32 — the contract. Environment: run gates via `devenv shell -- …` or
PATH cargo; never wait for confirmation. Territory: `crates/cix-cixfile/`, `examples/`
(all Cixfiles), `docs/cixfile.md`, and the single FROM/PKG row in `docs/docker.md`. Do NOT
touch `crates/cix/tests/` or `docs/tour/` (another track). COMMIT AS YOU GO. Work log:
`crates/cix-cixfile/LOG.md`.

1. Parser/compiler: remove the `PKG` directive; `${pkgs.<attrpath>}` resolves arbitrary
   attribute paths against the locked nixpkgs at build time. Bare `${name}` → error
   suggesting `${pkgs.name}`. `PKG` present → error explaining the D32 change (helpful, with
   the rewrite). Unknown attr → the nix eval error surfaced with the Cixfile line number.
2. Flip every Cixfile in `examples/` (incl. compose stack items, listenfds): delete PKG
   lines, prefix references with `pkgs.`. Rebuild all via `cix build`; run the sudo demos of
   nginx, postgres, and the compose stack; VM check.
3. Docs: `docs/cixfile.md` directives table (PKG row out; interpolation section rewritten:
   the `pkgs.` namespace, references-define-dependencies principle, closure as the manifest);
   the worked example updated; `docs/docker.md` FROM row's composix side updated to `pkgs.*`.
4. Tests: attrpath resolution (nested attr, unknown attr error with line number, bare-name
   suggestion, PKG-removal error), golden spec unchanged modulo nothing (resolution output
   identical to pre-D32 for same packages — prove with a fixture).

Gate: fmt/clippy/`cargo test --workspace` green ×2; the three sudo demos green; VM check
green; no leftover units; committed; clean status; LOG summary.
