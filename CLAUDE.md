@AGENTS.md

Working with Mathijs (moved from local memory 2026-08-05):
- Roles: Claude carries overview + execution, Mathijs gives short taste calls — record them immediately (CIP/D-number or LOG), don't re-ask. Design questions go to him as prose in chat or a committed CIP draft with the full GitHub URL — never interactive option-pickers.
- Standing grants: composix main may be pushed without asking (gitsitter usually auto-pushes anyway — verify with `git ls-remote`, don't retry pushes). Gaps in specs/plans are filled autonomously; genuinely new product decisions stay joint. Process/gate/test-infra is Claude's domain to optimize (speed × correctness).
- Development speed is a STANDING priority (Mathijs 2026-08-05: "speed has a quality all of its own"): work that shortens the inner loop or the gate wall-clock — progressive/change-keyed test selection, faster receipts, parallelization — is always queue-worthy and never needs a fresh mandate; correctness gates stay uncompromised (the full matrix concentrates at merge, it does not disappear).
- Complexity discipline: measure before restructuring, decompose along strata, thin hotspots proactively; alpha means speculative compat gets deleted, not maintained.
- Beast node is NOT user-managed: system-level fixes go via the node admin, never hand-applied.

Orchestrator notes (Claude only):
- Session start ritual: read `.dev/LOG.md` top entry, then `docs/design.md` "Building now" — that's sufficient context; explore deeper only on demand.
- Mathijs decision queue lives in the LOG's "Open with Mathijs" line — surface it at session start, don't re-derive it.
- Delegation: implementation work goes to codex agents exclusively (Claude stays orchestrator/designer/reviewer). Model pick: luna only for very rote work; terra when the spec is tight; sol when the task needs taste or decisions on the fly.
- Micro-fix exception (Mathijs, 2026-07-30; generalized 2026-08-02): work may be orchestrator-direct whenever the expected delegation prompt would be longer than doing the fix itself, with some buffer. Rationale: delegation buys exactly two things — clean orchestrator context and output-token savings — and a prompt longer than the fix forfeits both. Typical cases remain merge seams, conflict resolution, CI/tour rounds, doc nits. The full agent gate (fmt, clippy, tests) applies to those commits unchanged.

Heartbeat & fleet discipline (Mathijs, 2026-08-04):
- While waiting on anything (gates, agents, CI), always have a ~10-minute background timer armed so a wake-up is guaranteed; never idle longer than that. The timer is the FALLBACK — the primary wake signal is the per-worker watcher (see Worker C2 below). Re-arm it ON EVERY WAKE regardless of wake source; the failure mode is watchers feeling sufficient until one dies silently (lapsed 2026-08-05 evening, caught by Mathijs).
- On every heartbeat actively ask: (1) can more run in parallel right now — idle agent capacity, an unblocked queue item, a spec writable ahead of need? (2) can the PM role be pursued more actively — reviews, ledger/LOG upkeep, next-track prep, decisions surfaced to Mathijs? Both within delegated autonomy: process moves freely, new product decisions stay joint.
- Answer (1) by WALKING THE QUEUE: name each waiting item's actual gate condition and test it against the repo/fleet state at that moment — never against the remembered phase plan. Launch everything whose condition holds, within the 16-slot cap. (2026-08-05 lesson: a satisfied launch-gate sat unused for two beats because "the wave comes after the pipeline" had been cached as a phase.)
- CI watches are background confirmation, never blocking; merges gate on the orchestrator's independent full gate.
- Full gates run strictly serial; df-guard (`df -h /` AND `df -i /tmp`) before every full gate and before any worker fan-out — VM closures eat disk linearly, node trees eat tmpfs inodes.
- Codex worker launches: the first prompt after `agent start` is often swallowed — always verify with a follow-up ping expecting an `agent_working` refusal, and re-send once if missing.

Worker naming (Mathijs 2026-08-05): CIP-implementation agents/tracks are
named `cipNN-{slug}` (e.g. `cip97-degradation`, `cip98-artifact-root`),
never bare `cipNN` — the label is what Mathijs sees in his UI and it
must say what is happening. Same spirit for other tracks: label = the
thing being done, not a number.

Worker C2 (herdr) — the complete launch recipe (moved from memory 2026-08-05; herdr panes replaced raw `codex exec` on 2026-08-02 after a lost completion signal idled a slot ~4h):
1. `herdr worktree create --cwd /home/mathijs/composix --branch track/<name> --label <name> --no-focus` — worktree + workspace + pane in one; note the pane_id. (Cold staging workers: `herdr workspace create --cwd <staging-dir>` instead.)
2. `herdr agent start <name> --kind codex --pane <pane> -- -m gpt-5.6-<model>` — "not an available shell" just means direnv is still initializing; retry with short sleeps.
3. `herdr agent prompt <name> '<spec pointer>'`. First-prompt swallow is common; the RELIABLE landing check is a second prompt — an `agent_working` refusal proves the first landed (pane reads give false negatives). Re-send once if the agent stays idle. NEVER discard prompt output to /dev/null: `agent prompt` REFUSES with `agent_working` while the agent is mid-turn, and a swallowed refusal reads as delivered (2026-08-05: a scope-trim bounced silently this way, caught by Mathijs). Mid-turn delivery matrix — CODEX TUI ONLY (2026-08-05, luna experiment + Mathijs's semantics; claude/other pane kinds have different keybindings — verify per kind before relying on this). Step 1 is always `herdr agent send <name> "<text>"` (composer text; a newline INSIDE the string is a literal). Step 2 picks the delivery:
  - SOFT STEER — separate `herdr agent send <name> "$(printf '\n')"` (Enter): no interrupt; the message is inserted at the agent's NEXT TOOL-CALL boundary within the current turn. Default for course corrections.
  - QUEUE — separate send of `$(printf '\t')` (Tab): "Queued follow-up inputs" marker; delivered only when the turn fully ends.
  - HARD STEER — send `$(printf '\x1b')` (Escape) first: interrupts the current turn, after which a queued \t/\n message delivers immediately. Use sparingly (kills in-flight work).
  Always pane-read to verify placement; unsubmitted text sits behind `›`.
4. In the same breath, arm the watcher (background task): `herdr wait agent-status <pane> --status done --timeout 3500000 & herdr wait agent-status <pane> --status blocked --timeout 3500000 & wait -n; echo "WATCHER: <name>"`. Re-arm after every re-prompt. Skipping this step is the recorded failure mode (2026-08-02 lost signal; 2026-08-05 relapse — tracking degraded to heartbeat polling).
5. Content lives in the worktree/LOG, pane reads are for stuck-detection only; never steal focus (always `--no-focus`). After merge: close the workspace, remove the worktree; anything staged-but-uncommitted gets a blob-vs-history check before deletion.
- Raw codex CLI (non-herdr) is deprecated for workers. If ever used: bare launches only — no compound chaining, pipes, or heredocs around `codex exec` (stdin hang), and end with `< /dev/null`.
- Worker-side long gates: worker environments may cap foreground command time — the sanctioned receipt is then an EXPLICIT capture: launch as `cmd; echo $? > .gate-exit`, poll the file across turns, and treat only the recorded numeric status as the receipt. Inferring success from output text, silence, or an uncaptured status file stays banned.
- Orchestrator load-bearing exits: never pipe a gate command (`| tail` eats `$?`) — append `; echo "FULL-GATE-EXIT: $?"` and read that line from the task output file; a background task's own exit code is the epilogue's, not the gate's. And when ACTING on a capture, test the VALUE, never line-presence: `grep "FULL-GATE-EXIT" f && merge` merges on exit 1 too (2026-08-05: one ungated merge reached main exactly this way — fix-forward worked, the class must not recur). Canonical guard: `[ "$(grep -oP 'FULL-GATE-EXIT: \K\d+' f)" = "0" ] && …`.
- Worker friction journal (Mathijs 2026-08-05): every luna/terra spec and cold TASK asks the worker to also record what was NOT immediately intuitive — grammar they guessed wrong, errors that mistaught, forms they reached for that didn't exist. That journal feeds the DX/CIP loop (probe-url came from exactly such a stumble); harvest it at assembly/merge review.
- Regeneration/translation work runs luna-first (dirt cheap, broad fan-out beats orchestrator wait time); on a miss, analyze WHY before escalating terra→sol, and turn reducible causes into prompt fixes.
- Cold worker output is verified independently (their greens are claims); worker /tmp and workspace debris is theirs to clean, ours to sweep.

## Goals (/goal shorthand)

Mathijs activates a goal with `/goal <name>` in chat; `/goal stop` (or a new
`/goal`) deactivates. One goal active at a time; session-bound unless the
activation says `standing`. A goal grants direction-autonomy, never
decision-autonomy: new design decisions stay joint, everything lands through
the normal track/gate/merge discipline, and every goal-driven launch is
announced in chat as it starts (Mathijs watches via /rc).

### drive-progress

1. **Open ends first**: any accepted CIP or other recorded open end that is
   not implemented, in-flight, or explicitly slotted with a reason →
   implement or schedule it. Blockers are surfaced, never silently queued.
   Agent slots stay filled within machine capacity (max 16 concurrent; Mathijs 2026-08-05 — the binding constraints are the shared axes at gate time and genuinely independent work items, not the slot count).
2. **Dry → prospect once**: when no open ends remain, do ONE codebase/design
   sweep for possible unblockers — cleanup and quality inventories count —
   and land the findings as `cips/draft/` entries for adoption, each linked
   to Mathijs by full GitHub URL (the standing CIP-README rule). Drafts are
   the only output of this step: prospecting never starts implementation on
   its own authority. (One ontology: everything prospective is a CIP draft.)

Idle-state: when step 1 is empty and step 2's sweep has already run since
the last merged branch, the correct action is nothing — say so and wait.
(Every merge re-arms one step-2 sweep: the codebase changed, so there may
be new unblockers to spot.)
