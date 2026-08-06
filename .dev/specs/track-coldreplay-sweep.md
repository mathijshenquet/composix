# track/coldreplay-sweep — source-compile locks under the widened self-write parser

Context: the trace self-write hygiene completed on 2026-08-06 (valkey fix
+ httpd symmetric completion, merges `9a6f9bcd` and the valkey/httpd LOG
entries): three scheduling classes are now suppressed symmetrically over
warm and cold (pre-create probes, clone-resumed cwd, truncating creates).
Other source-compile locks were generated under the OLD parser and may
retain now-suppressed observation classes — their next cold replay would
mismatch exactly like httpd's did.

Cases, in `corpus/migrate/docker/`: redis, mosquitto, memcached, nginx,
haproxy, tomcat, valkey (its post-fix lock — verify, don't assume).

Per case, in order:
1. **Cold-replay verify** under current main: run the case's cold replay
   per its receipt.md command; capture the synchronous exit status.
2. Exit 0 → record "cold replay verified under widened parser
   <date>" in receipt.md; move on. No lock churn expected — if the lock
   dirties on a verify, STOP for that case and record the diff verbatim
   (that is a keying-neutrality exhibit, not something to paper over).
3. Mismatch → from-scratch regenerate (the httpd recipe: full re-lock,
   then warm+cold+runtime all captured exit 0), record the lock
   line-count delta in receipt.md, regrade GAPS.md.
4. Evidence-invalid warm/cold pairs (inherited output/, memo-hit
   masquerading as a build) are discarded and redone — the httpd
   standard.

Close with a whole-corpus lock hash diff proving only regenerated cases'
locks changed, and the corpus suite green.

Walls (network, upstream drift) are valid outcomes — record honestly in
GAPS.md and continue with the remaining cases.

Discipline: branch `track/coldreplay-sweep`, LOG `crates/cix/LOG.md`
(append, timestamped, FRICTION section). Receipts: synchronous
value-checked exit codes only; exact repro commands in the LOG. Gates:
fmt + corpus suite; no Rust changes expected (if a product bug surfaces,
record it and stop — do not fix the parser on this track). Clean
committed branch; do not merge.
