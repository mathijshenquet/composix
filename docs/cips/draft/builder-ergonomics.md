# Builder ergonomics — import env, lock metadata, and the no-op floor

Status: **draft** (2026-08-02, prompted by the gitsitter compare Cixfile
review with Mathijs). Three small taste calls plus one measured
mechanical fix; batch-decidable.

## 1. The problem

The gitsitter compare Cixfile carries three frictions that are not the
author's business:

```dockerfile
ENV GIT_COMMIT_HASH = 29c8a2dede19b5e7d1bd7e65f81829fa0ac66ecd   # (b)
FETCH cargo vendor --locked vendor > cargo-vendor-config.toml && rm -rf .cargo   # (c)
RUN mkdir -p .cargo && cp cargo-vendor-config.toml .cargo/config.toml            # (c)
RUN PKG_CONFIG_PATH=${pkgs.openssl.dev}/lib/pkgconfig:...:... cargo build ...    # (a)
```

And measured 2026-08-02: the hot no-op costs **1.13 s** against the
upstream flake's 0.07 s (d).

## 2. Prior work

- (a) **nixpkgs stdenv setup hooks** auto-derive `PKG_CONFIG_PATH` (and
  much more) from `buildInputs` — the right instinct buried in the
  machinery this project avoids. pkg-config is *the* discovery protocol
  for cargo's `-sys` crates; its `.pc` files live in nix `.dev` outputs,
  and IMPORT deliberately unions only `bin`/`etc`/`share`, so the
  imported pkg-config has no default path that could ever work.
- (b) **flakes** expose `self.rev` / `self.shortRev` /
  `self.dirtyShortRev`; the upstream gitsitter flake uses exactly that
  for its version stamp. Cixfile.lock already pins the FROM binding's
  rev + narHash — the information exists, it just has no spelling.
- (c) The `.cargo` dance predates consume-narrowed FETCH pins
  (docs/nix-build.md records its origin: `cargo fetch`'s Cargo home was
  not byte-stable). Since the underlay round, "incidental cache files
  outside the consumed set do not make a build flap" — the reason
  should be dead.
- (d) The skeleton's one-alias philosophy (`/usr/bin/env` only) is the
  precedent for minting exactly one blessed thing when evidence
  demands, and nothing speculatively.

## 3. Recommendation

- **(a) Auto-`PKG_CONFIG_PATH`**: for each IMPORT in declaration order,
  if `<pkg>/lib/pkgconfig` exists, append it. Earlier-wins ordering
  matches the IMPORT collision rule. No other variables are minted
  until a corpus case demands them (candidates like `CMAKE_PREFIX_PATH`
  wait for evidence, logged in the ledger when refused).
- **(b) Lock metadata attributes on FROM bindings**: `${src.rev}`,
  `${src.shortRev}`, `${src.narHash}`, resolved from Cixfile.lock at
  parse time; part of the resolved arguments, so keying is automatic
  and `--update-lock` moves the stamp. On a binding with no rev (local
  `FROM . AS src`, dirty trees) referencing `${src.rev}` is a spanned
  error that names the fix (write a literal or use a clean ref) — the
  flake `self.rev` refusal, honestly reproduced.
- **(c) Kill the `.cargo` dance**: acceptance check that the fixture
  without `rm -rf .cargo` + restore converges normal/repeat/`--cold`.
  If workspace junk still bites anywhere, the documented idiom is
  `ENV CARGO_HOME = /tmp/cargo` (junk lands outside the snapshot);
  the dance is never taught again either way.
- **(d) No-op floor** (mechanical, recorded here so the regression
  assertion has a home). Measured on a full-memo-hit no-op, cix spawns
  **11 nix subprocesses** (486 execve attempts including PATH probing):
  1× `nix eval --impure builtins.currentSystem`; 1× eval + 2× build of
  `builtins.path` source-import expressions; 5× `nix hash path` on
  *immutable* store paths; 2× `nix store add` — one of which re-NARs
  the unchanged output item. All eliminable on the hit path: determine
  the system in-process; memoize store-path hashes (they cannot
  change); stat/validity-check the memoized item instead of re-adding;
  batch whatever remains. Target: ≤0.15 s, and structurally **zero
  subprocesses and zero steps executed on a no-op** — that, not
  wall-clock, is the CI assertion (via the `--stats` channel from the
  read-set-keying draft).

## 4. Open questions

1. (b) Which attributes exactly — is `lastModified` wanted, or do we
   mint only rev/shortRev/narHash until asked?
2. (a) Should the refused-for-now env candidates get docker.md-style
   ledger rows, or is a code comment enough?
3. (c) Is a lint warranted when a FETCH leaves more than N MiB of
   unconsumed junk in the workspace, or does the consumed-set model
   make junk fully free forever?

## Placement of fixture and assertions (shared with read-set draft)

- `examples/compare/gitsitter` stays the demonstration + benchmark
  fixture (examples are e2e-verified; wall-clock numbers remain dated
  receipts in docs/nix-build.md, never CI-asserted).
- Mechanics regressions live in the cargo test tier on a hermetic
  mini-fixture (FETCH against a test-local HTTP server): no-op executes
  zero steps / zero subprocesses; the (c) convergence check; (a) and
  (b) get ordinary unit + golden coverage.
