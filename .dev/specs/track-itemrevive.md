# track/itemrevive — D68: ITEM as manifest-less pure store tree

Read AGENTS.md first. Authoritative: design.md **D68** (and D67 for the seam
language). Scope: crates/cix-cixfile (parser/codegen), crates/cix-run (run
error on manifest-less items), examples (one small ITEM example), docs
(cixfile.md; migrate.md where relevant), tour. Runs parallel to
track/coldaudit (crates/cix tests only) — coordinate nothing, files disjoint;
do NOT touch crates/cix/tests/cold_audit.rs if it appears.

1. Parser: `ITEM <name>` block returns (replace the D50 migration error).
   Allowed: COPY/FILE/LINK only; every stratum-1a directive inside ITEM = the
   D68 seam error ("items are build products; SERVICE/APP declare runnable
   contracts"). Block name = member name (D62 rules apply: selector, tagging,
   JSON map).
2. Codegen: an ITEM emits its assembled tree with NO cix-manifest.json.
   D66/D64 mechanics apply to spelling only where meaningful (absolute
   destinations; there is no runtime PATH — nothing runs).
3. cix run / exec / debug on a manifest-less store path: clear error naming
   the seam and pointing at SERVICE/APP. `cix build .#item`, `-t` tagging,
   and D65 `FROM <item-ref> AS x` consumption must all work — add a real-nix
   test: ITEM producer → tag → FROM-consume → COPY out → build green.
4. Example: examples/build/item (small pure-asset tree) with README; sweep
   docs/cixfile.md block-kind table and migrate.md's block table (ITEM row:
   "pure store tree, no manifest").
5. Gate: fmt / warning-denied clippy / workspace tests / tour regen + drift +
   determinism twice / vm-dogfood. Exact repros + cleanup in
   crates/cix-cixfile/LOG.md. Commit on this branch when green.
