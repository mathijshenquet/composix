# track/cip101-livelock — sweep must never reap live scratch (main-CI red)

Context: CI on main (run 31041344781) fails
`cix::fetch_probe_cleanup::sigterm_removes_live_build_scratch`:
"cix did not create scratch under $TMPDIR/temp", then a concurrent cix
errors "No such file or directory" creating scratch — the tightened
CIP-101 startup sweep (expand2's amendment d3e61a9: hours + size-aware)
can reap scratch belonging to a LIVE build. The old 1-day threshold
protected live builds only by accident; the aggressive threshold makes
the hole structural — CI's slower timing exposes it, the logic is
wrong on every host.

Fix: liveness-guarded sweeping. Each scratch root records its owner
(e.g. a lockfile held with flock by the owning process, or pid+boot-id
marker verified alive); the startup sweep skips owned/live roots
regardless of age/size and reaps the rest per the tightened policy.
Keep the aggressive reaping for genuinely dead scratch — that fix was
right, it just lacked the liveness predicate. Regression tests: (1)
a live build's scratch survives a concurrent cix invocation's startup
sweep; (2) dead (unlocked) scratch is still reaped promptly; (3) the
existing sigterm test passes under an artificial slow-start (inject
latency or reduce the race window deterministically rather than
sleeping-and-hoping).

Append the dated changelog line to cips/accepted/0101-tmp-relocate.md.

Discipline: branch `track/cip101-livelock` (from current main), LOG
`crates/cix-build/LOG.md`; full agent tier (the failing test class is
cargo-tier — run the workspace suite with attention), value-checked
capture receipts, bounded VM parallelism. Clean branch; do not merge.
