# track/noscript — D55 SCRIPT removal

## Contract

Implement D55 as a hard language removal:

- remove `SCRIPT` from the Cixfile model, parser, Nix generation, and tests;
- reject every `SCRIPT` directive with the exact line-numbered migration message:
  `SCRIPT was dropped (D55); COPY a real script and EXEC ${pkgs.bash}/bin/sh <path>, or use FILE if the content needs store-path interpolation`;
- keep `FILE` unchanged;
- migrate live example Cixfiles to checked-in scripts;
- rewrite tour chapters 3 and 6 so their visible input trees contain real script files,
  their Cixfiles COPY those files, and their manifests invoke an explicit nixpkgs shell;
- regenerate the committed tour and remove stale `SCRIPT` claims from active documentation.

Historical work logs and superseded track specifications remain untouched. D55 itself remains
in `docs/design.md`; earlier design prose is updated where it otherwise describes the current
directive set.

## Gate

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p cix --test tour -- --ignored generate_tour
git diff --exit-code -- docs/tour
cargo test -p cix --test tour tour_matches_committed_document -- --exact
cargo test -p cix --test tour generated_tour_is_deterministic -- --exact
cargo test -p cix --test tour generated_tour_is_deterministic -- --exact
nix build .#checks.x86_64-linux.vm-dogfood --no-link
```
