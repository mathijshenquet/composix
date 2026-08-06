# track/valkey-coldtrace — cold replay divergence on build-generated temp files

corpus/migrate/docker/valkey GAPS.md records the exhibit: the
faithful 8.1.9 build and `valkey-cli PING` pass warm, but an
empty-workspace `--cold` replay exits 1 while compiling
`libbacktrace/.libs/stVX6SFe`: the warm trace recorded that path as
`Some(Absent)` and the cold trace records `None`. Attribution is
→ language (CIP-87 read-set keying).

The class, not the case: `stVX6SFe` is a RANDOM libtool temp name —
a file the build itself creates and probes mid-compile. Warm and
cold runs generate DIFFERENT random names, so any keying of
build-generated ephemeral paths can never replay stably. Reproduce
first (the case is committed; `cix build corpus/migrate/docker/valkey
--cold` style repro — read the case receipt for exact commands),
then diagnose precisely where the warm trace admits the probe into
the read-set and why cold classifies it differently.

IMPORTANT — decision boundary: a fix here touches read-set/memo
SEMANTICS (what counts as an observation). Diagnose fully and
implement the fix you judge correct ONLY if it is clearly
observation-classification hygiene (e.g. excluding reads of paths
the same build step itself created — self-writes are outputs, not
inputs). If the correct fix requires a genuine semantics choice with
alternatives (new lock field, trace format change, directive
change), STOP after diagnosis: write the analysis and options as
`cips/draft/coldtrace-selfwrites.md` (four-chapter form: Problem /
Proposal / Effort / open Decision) and leave implementation out.
Semantics decisions are joint with Mathijs.

Verification if you do fix: valkey warm + cold replay both exit 0
synchronously; the whole corpus suite stays green; no other case's
lock churns (byte-diff the locks). Regrade valkey GAPS.md honestly.

Discipline: branch `track/valkey-coldtrace` from current main, LOG
`crates/cix-build/LOG.md` (append, FRICTION section). Full agent
tier if Rust changes; corpus receipts either way. Value-checked
synchronous captures only. Clean committed branch; do not merge.
