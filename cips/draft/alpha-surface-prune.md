# Alpha surface prune — retire only evidence-free compatibility

Status: **draft, CIP-light** (2026-08-05; follow-up to CIP-89's compatibility
audit).

## Problem

The alpha rule is reject-and-teach, not silently accept old behavior. Current
zero-exhibit surface includes 254 lines of commented extracted code, 148 lines
of permanently disabled tests, three legacy `MemoEntry` fields,
`FetchPin::store_path`, `--no-cache`, and formatter-only leading
`FETCH EXPECT` acceptance. None of those lock fields occurs in its legacy
position in 44 committed locks. Conversely, ten `LINK` directives remain in
seven active Cixfiles and 18 whole-tree FETCH pins remain in nine locks, so
those readers still have in-repo evidence.

## Proposal

1. Delete commented/disabled code and the superseded runtime `ps` path.
2. Delete zero-exhibit legacy lock fields/read branches and the `--no-cache`
   alias. Drop formatter-only old syntax unless a current migration policy is
   explicitly chosen.
3. Mechanically migrate the seven active LINK-using Cixfiles, then remove LINK
   acceptance in the same track.
4. Regenerate the nine whole-tree-lock exhibits before deleting whole-tree
   FetchPin support; do not delete it on assertion alone.
5. Keep removed-directive, obsolete-field, and old-index rejection diagnostics:
   they accept no old behavior and teach recovery.

## Effort

**S/M.** Zero-exhibit deletion is S. LINK and lock regeneration make the full
track M and require focused corpus/example receipts.
