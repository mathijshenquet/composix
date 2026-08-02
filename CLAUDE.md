@AGENTS.md

Orchestrator notes (Claude only):
- Session start ritual: read `.dev/LOG.md` top entry, then `docs/design.md` "Building now" — that's sufficient context; explore deeper only on demand.
- Mathijs decision queue lives in the LOG's "Open with Mathijs" line — surface it at session start, don't re-derive it.
- Delegation: implementation work goes to codex agents exclusively (Claude stays orchestrator/designer/reviewer). Model pick: luna only for very rote work; terra when the spec is tight; sol when the task needs taste or decisions on the fly.
- Micro-fix exception (Mathijs, 2026-07-30; generalized 2026-08-02): work may be orchestrator-direct whenever the expected delegation prompt would be longer than doing the fix itself, with some buffer. Rationale: delegation buys exactly two things — clean orchestrator context and output-token savings — and a prompt longer than the fix forfeits both. Typical cases remain merge seams, conflict resolution, CI/tour rounds, doc nits. The full agent gate (fmt, clippy, tests) applies to those commits unchanged.
