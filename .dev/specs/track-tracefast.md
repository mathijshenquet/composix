# track/tracefast — CIP-87's warm-edit bar: 84.83s → ≤9s

Read AGENTS.md first (focused agent gate; synchronous receipts;
shared-state rule). Authoritative: cips/accepted/0087-read-set-keying.md
including the NEW performance-criterion changelog entry (Mathijs:
~8–9s warm one-line edit on the gitsitter compare = green; above =
ORANGE; failing to beat crane's 16.46s puts the CIP itself in
question). Work in
`/home/mathijs/worktrees/composix/track-tracefast` (herdr worktree) on
branch `track/tracefast`. Keep `crates/cix-cixfile/LOG.md` current.
PARALLEL FENCE: track/hygiene-a runs concurrently in the same crates
but owns env/config plumbing (clap boundary, fetch consent env); you
own crates/cix-build/src/{trace.rs,build_chain.rs memo/validation
paths} and the measurement harness. Do not touch env/config intake.
track/netns owns cix-compose — out of scope.

1. **Measure first, attribute completely** (the vmslim method): break
   the 84.83s down with synchronous receipts — (a) ptrace capture
   overhead during the RUN (run the same cargo build with and without
   tracing), (b) memo-lookup validation re-hashing (how many bytes/
   files re-hashed on the warm path, wall time), (c) COPY staging of
   the whole source, (d) chain-key source hashing, (e) each of the 11
   nix subprocesses individually timed (this table doubles as the ROI
   input for the libnix draft decision — label it as such in the LOG).
   No optimization lands before its cost is on this table.
2. **Optimize, biggest first.** Candidates (measure-gated, not
   mandatory): validation fastpath — a (dev,inode,mtime,size,len)
   cache so memo-hit validation and chain hashing skip re-hashing
   unchanged files (§5.6 explicitly allows this); trace-capture cost —
   syscall-filtered ptrace (opens/stats only), or fanotify/seccomp-
   notify fastpath IF it preserves §3's completeness (negative
   lookups included — completeness is not negotiable unilaterally);
   staging via hardlink/reflink instead of byte copies; batching or
   eliminating the measured-expensive subprocesses.
3. **The bar**: re-run `examples/compare/gitsitter/measure-warm.sh`
   after each landed optimization; the LOG carries the descent table.
   Green = ≤9s; if after honest effort the floor with COMPLETE
   capture sits above that, STOP and report the exact
   completeness-vs-speed frontier (what each relaxation would buy) —
   that decision is Mathijs's, per the CIP.
4. **Guardrails**: no-op stays ≤0.4s with zero subprocesses (receipt);
   cold unchanged within noise; every CIP-87 acceptance test stays
   green byte-identically (the hermetic suite is the correctness
   proof); the nix-build.md receipts update with dated numbers.

Gate (agent side): fmt / examples fmt / warning-denied clippy / full
workspace tests / tour regen + drift / focused: vm-dogfood +
measure-warm.sh receipts. Full matrix at the orchestrator gate.
Commit on this branch when green.
