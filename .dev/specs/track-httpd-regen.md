# track/httpd-regen — regenerate httpd with self-write trace hygiene

corpus/migrate/docker/httpd/GAPS.md says
`Status: stale — regenerate with self-write trace hygiene`: httpd
exhibits the same generated-path class that track/valkey-coldtrace
just fixed (exclusive creates classified as outputs, not inputs;
same-step generated paths suppressed from the read set).

Do: from-scratch re-lock (`--update-lock build` style — read the
case receipt.md for the exact command), synchronous captured exit 0;
verify the cold replay now also exits 0; run check.sh for the
runtime probe; record the lock line-count delta in receipt.md;
regrade GAPS.md (drop the stale marker, state what is verified);
regenerate the corpus browser and keep the corpus suite green.
Confirm with a whole-corpus lock hash diff that ONLY httpd's lock
changed.

If regeneration hits a wall (network, upstream drift), record it
honestly in GAPS.md and stop — an honest wall is a valid outcome.

Discipline: branch `track/httpd-regen` from current main, LOG
`crates/cix/LOG.md` (append, FRICTION section). Gates: fmt + corpus
suite; `cargo test --workspace` once only if Rust changes (expected:
none). Value-checked synchronous captures only. Clean committed
branch; do not merge.
