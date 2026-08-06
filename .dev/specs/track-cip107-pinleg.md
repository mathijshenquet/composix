# track/cip107-pinleg — CIP-107 remaining leg: whole-tree pin exhibits + FetchPin deletion

Read cips/accepted/0107-alpha-surface-prune.md. Every leg has landed
except proposal item 4: 18 whole-tree FETCH pins remain in nine
committed locks, and whole-tree FetchPin support must not be deleted
on assertion alone.

Order is the CIP's own: FIRST regenerate the nine whole-tree-lock
exhibits so their locks carry current (subtree/aggregated, CIP-99
era) pin forms — find them with a repo-wide grep for the whole-tree
pin form in *.lock / Cixfile.lock files; each regeneration needs a
synchronous captured receipt (build exit 0, lock diff reviewed).
THEN delete whole-tree FetchPin read/write support and its tests,
keeping the reject-and-teach diagnostic per CIP-107 item 5 (a
rejected old lock must name what changed and how to regenerate —
CIP-102 mismatch diagnostic style).

If any exhibit CANNOT regenerate cleanly (network wall, upstream
drift), record it honestly in its GAPS.md and STOP before the
deletion step — deletion only happens when zero committed exhibits
remain. That stop is a valid track outcome; report it.

Discipline: branch `track/cip107-pinleg` from current main, LOG
`crates/cix-build/LOG.md` (append). Full agent tier: fmt / examples
fmt / clippy -D warnings / full workspace tests / tour regen+drift /
progressive VM check; corpus receipts for every regenerated exhibit.
Value-checked synchronous captures only. FRICTION section in the
LOG. Clean committed branch; do not merge.
