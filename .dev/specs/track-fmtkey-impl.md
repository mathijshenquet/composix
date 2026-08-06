# track/fmtkey-impl — implement CIP-110: canonical-AST keys, NAR-invariant fingerprints

Charter: `cips/accepted/0110-fmt-key-neutrality.md` — the Decision
chapter is the contract; the evidence table's rightmost column is the
site-by-site spec. The hermetic characterization tests from
fmtkey-evidence pin CURRENT behavior — flip them into regression tests
of the NEW behavior as you land each site.

Scope, in order:

1. **NAR-invariant fingerprint primitive** (one function, one place):
   object identity = type + content + executable bit + symlink target.
   Replace at every table site: `trace::read_hash`,
   `trace::directory_hash`/`filesystem_subtree_hash`,
   `fetch_state::file_fingerprint`, and fix the source-tree hash's
   missing-executable-bit gap. `trace::file_fingerprint` becomes an
   unkeyed validation hint only (or is removed if that is cleaner).
2. **Canonical-AST key serialization**: `build_fingerprint` and every
   semantic key derive from the parser's canonical form, never raw
   text. Own this as a named API seam (a public `canonical` module or
   equivalent) — track/epoch-groundwork will consume it for the new
   grammar; keep the surface small and documented.
3. **The CIP's acceptance fixture**: lock a deliberately non-canonical
   Cixfile, cold-replay it, `cix fmt` a copy, assert identical FETCH
   identities, snapshot lookups, and item output.
4. **Honest identity versioning**: old raw-text-derived identities do
   not silently collide with new ones — version the key computation
   (the CIP: "version/fingerprint the old raw-text identities
   honestly"). Existing corpus locks will mismatch under new keys:
   that is EXPECTED and stays unswept — the epoch sweep (a later
   track) regenerates the corpus once. Do NOT regenerate corpus locks
   here; keep the corpus suite green by whatever the suite actually
   requires (if the suite hard-depends on lock validity against new
   keys, stop and report — that is a sequencing decision).

Gates: fmt, examples fmt, warning-denied clippy,
`cargo test --workspace`, tour regen+drift if touched, focused VM via
`devenv shell -- nix run .#progressive-vm-check`.

Discipline: branch `track/fmtkey-impl`, LOG `crates/cix/LOG.md`
(append, timestamped, FRICTION section). Synchronous value-checked
receipts only. Clean committed branch; do not merge.
