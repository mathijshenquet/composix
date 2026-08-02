# track/ergo — CIP-88: builder ergonomics + the no-op floor

Read AGENTS.md first. Authoritative: docs/cips/0088-builder-ergonomics.md
(§3 + §5 Decision). NOTE: track/devices runs concurrently and touches
the cix-cixfile parser (new CLAIM/SHM directives) — your parser changes
are the FROM-attribute interpolation only; keep them isolated so the
merge seam stays small. CIP-87 (read-set keying) is adopted but NOT this
track — do not restructure the memo model; build the `--stats` channel
against today's chain keying (CIP-87's track will extend it). Work in
`.worktrees/ergo` on branch `track/ergo`. Keep
`crates/cix-cixfile/LOG.md` current.

1. **`cix build --stats`**: machine-readable JSON to stdout alongside
   the normal result — per step: name/kind, executed | memo-hit; plus
   total nix-subprocess count for the invocation. This is the assertion
   channel; keep the schema minimal and stable.
2. **Auto-`PKG_CONFIG_PATH`** (CIP-88 §3a): for each IMPORT in
   declaration order, if `<pkg>/lib/pkgconfig` exists, append it to the
   step environment. Earlier-wins order. Record the refusal of further
   env candidates as a code comment at the site (§5.2).
3. **Lock metadata attributes** (§3b): `${src.rev}`, `${src.shortRev}`,
   `${src.narHash}` on FROM bindings, resolved from Cixfile.lock at
   parse time, part of resolved arguments (keying automatic). Spanned
   error when the binding has no rev (local dir, dirty tree) naming the
   fix. Nothing else minted (§5.1).
4. **No-op floor** (§3d): eliminate the measured 11 nix subprocesses on
   a full-memo-hit no-op — in-process system detection (no
   `nix eval builtins.currentSystem`), memoized hashes for immutable
   store paths (never rehash), existence/validity check instead of
   `nix store add` re-adding unchanged outputs, batch what remains.
   Structural goal: **zero subprocesses and zero steps executed on a
   no-op**; wall-clock target ≤0.15 s as a dated receipt, not a CI
   assertion.
5. **Kill the `.cargo` dance** (§3c): simplify
   `examples/compare/gitsitter/cix/Cixfile` — drop the
   `rm -rf .cargo` + restore RUN, drop the manual PKG_CONFIG_PATH
   (now automatic), replace the hardcoded `ENV GIT_COMMIT_HASH` with
   `${src.rev}`. Acceptance: normal / repeat / `--cold` converge
   byte-identically on the simplified fixture. If junk still bites,
   the documented idiom is `ENV CARGO_HOME = /tmp/cargo` — the dance
   is not taught either way.
6. **Hermetic mini-fixture** (CIP-87/88 regression surface): a tiny
   builder project in the cargo test tier whose FETCH pulls from a
   test-local HTTP server (pattern exists in the index/tour tests).
   Assertions via `--stats`: no-op → zero steps executed, zero
   subprocesses; a source edit re-executes the affected steps;
   normal/repeat/`--cold` byte-converge. Wall-clock is never asserted.
7. **Docs**: docs/cixfile.md — auto-PKG_CONFIG_PATH under IMPORT, the
   `${src.rev}` family under FROM, `--stats`; docs/nix-build.md — update
   the quoted Cixfile listing + LOC row honestly, re-measure ONLY the
   cix no-op receipt (dated, same host caveats), leave other measured
   numbers untouched with their original dates.

Gate: fmt / `cargo run -- fmt --check examples` / warning-denied
clippy / workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Exact repros in the LOG. Commit
on this branch when green.
