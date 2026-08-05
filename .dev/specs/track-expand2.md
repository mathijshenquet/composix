# track/expand2 — expansion wave 2 assembly (valkey, haproxy, httpd, mosquitto)

Per the expand1 pattern: assemble the four staging outputs at
`/home/mathijs/regen-stage/new-{valkey,haproxy,httpd,mosquitto}` into
`corpus/migrate/docker/<case>/` as NEW cases (corpus 24 -> 28):
Cixfile(+locks), Dockerfile, SOURCE, check.sh, GAPS from NOTES,
receipt from your INDEPENDENT re-verification (worker greens are
claims; value-checked captures). Extend fetch.sh coverage; new rows in
docs/corpus.md (new ribbon vocabulary); CANDIDATES consumed; browser
regenerated (note: artifact renders are capped at 10k lines now).

Promote the staging findings — this harvest is a deliverable:
1. **fmt-key-neutrality** (haproxy): `cix fmt` re-indentation changed
   the builder identity/chain key, forcing a refetch. Formatting must
   be key-neutral (keys over canonical form). Write
   `cips/draft/fmt-key-neutrality.md` (CIP-light) with the exact
   repro from haproxy's NOTES — this touches declared-text-keying
   fundamentals, cite D59/CIP-87 context.
2. **valkey cold read-set mismatch at libbacktrace**: route as a gap
   bullet with the precise diagnostic (→ evidence/language per your
   judgment) — the source-compile-with-debug-symbols class.
3. **httpd sandbox findings**: /dev/stdout-/proc/self/fd unavailable
   under the service sandbox (LOGDIR is the working form — a
   migrate.md teaching line), and the stale-GPG-socket-in-workspace
   hazard (CIP-101-adjacent workspace hygiene — gap bullet or draft
   per weight).
4. All FRICTION sections harvested into draft evidence or LOG.

Discipline: branch `track/expand2` (branched from CURRENT main — the
history was rewritten today; never merge any pre-rewrite ref), LOG
`corpus/migrate/LOG.md`; full agent tier, value-checked
capture-as-epilogue, bounded VM parallelism, df-guard before builds.
Clean branch; do not merge.
