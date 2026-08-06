# track/expand-ntfy-filebrowser — two new corpus cases with artifact prestaging

Add corpus/migrate/docker/ntfy and corpus/migrate/docker/filebrowser,
following the existing case anatomy exactly (read docs/corpus.md "How
this corpus is maintained" and copy the structure of a recent green
case such as mailpit): Dockerfile (upstream's, faithful), SOURCE
(provenance + chosen tag), check.sh (runtime probe contract),
Cixfile + Cixfile.lock, receipt.md, GAPS.md, context via
corpus/migrate/fetch.sh.

Known requirement from the prior expansion band: both upstreams
install a prebuilt release artifact (ntfy: release tarball;
filebrowser: install script fetching a release binary). The faithful
translation must PRESTAGE those artifacts as explicit FETCH inputs
with pins — no install-script network access at build time. If an
artifact genuinely cannot be pinned/fetched reproducibly, that is an
honest wall: record it in GAPS.md with the exact failing form, and
keep whatever partial case is real (a building faithful twin with an
unproved runtime is still a case — grade it honestly).

Per case: faithful build receipt (synchronous exit 0), runtime probe
via check.sh where achievable, dissolved twin per the standard
contract, honest GAPS.md (deviation → {language|case|evidence}
attribution), corpus.md rows added with desk-vs-verified grades
distinguished. Cold-stage compatibility matters: the case must work
under corpus/migrate/regen-stage.sh conventions (upstream-* files,
context/).

Discipline: branch `track/expand-ntfy-filebrowser` from current
main, LOG crates/cix/LOG.md (append). Gates: corpus receipts +
`cargo test --workspace` once if any Rust is touched (should be
none); no VM matrix for corpus-only work. FRICTION section in the
LOG: record every Cixfile-language form you reached for that did not
exist or misbehaved — prestaging exercises FETCH ergonomics and that
journal feeds the DX loop. Value-checked synchronous captures only.
Clean committed branch; do not merge.
