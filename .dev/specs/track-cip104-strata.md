# track/cip104-strata — execute CIP-104 (crate strata, D67/D73)

Read cips/accepted/0104-crate-strata.md — it is the authoritative
scope, and its Decision section pins the acceptance bar: acyclic
crate graph; manifest/codegen cross-checks enabled (the currently
disabled codegen tests come back to life); runner library compiles
without index; parsing/formatting compiles without compose; generated
manifests and CLI behavior BYTE-IDENTICAL.

Prior context: CIP-103 just completed — build_chain.rs is 1051 lines
with Workspace/MemoEngine/NixEvaluation/FetchState/Sandbox owners
(request-result seams). Follow that same style: narrow nameable
interfaces, no bag-of-fields contexts, no unjustified shared
ownership (repo rule: every Arc/Mutex/RefCell/static needs a stated
site-local justification comment).

Sequencing advice (not mandatory): (1) stratum-1a neutral manifest
crate first, byte-identity receipts after each move; (2) codegen out
of cix-build; (3) cix-cixfile drops its compose dependency by moving
build/watch CLI coordination up; (4) resolver injection last. Commit
per stage so a partial landing is reviewable. Byte-identity receipt:
render all example + corpus manifests before/after each stage and
diff — record the diff command and its exit in the LOG.

If you hit a genuine design fork the CIP does not settle (e.g. where
the neutral crate's boundary cuts a type in half), write the fork and
your recommendation in the LOG, pick the conservative option, and
continue — flag it for merge review rather than blocking.

Discipline: branch `track/cip104-strata` from current main, LOG
`crates/cix-build/LOG.md` (append, timestamped, with a FRICTION
section). Full agent tier:
fmt / examples fmt / clippy -D warnings / full workspace tests /
tour regen+drift / progressive VM check. Value-checked synchronous
captures only; a receipt is an exit status you observed. Clean
committed branch; do not merge.
