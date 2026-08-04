# track/netnsrace — reproduce and root-cause the netns activation race under load

Read AGENTS.md first (gate convention; synchronous receipts). Work in the
herdr worktree on branch `track/netnsrace`. Keep `crates/cix-run/LOG.md`
current under a dated `track/netnsrace` heading.

The recorded open item (docs/open-questions.md "Open for agents"): during
the 2026-08-02 orchestrator gate, the scenario-netns closed-root leg failed
once under full parallel VM load — `cix-netns-b-fixed.service` failed
during activation — and passed focused on the identical tree. Suspected:
a real ordering race (member starting before the pod netns oneshot
completes) that only load exposes. Mandate: reproduce under contention and
root-cause before waving it off as flake.

## Deliverables

1. **Reproduction evidence**: run the netns scenarios repeatedly under
   deliberate contention (parallel scenario instances and/or constrained
   cores — bound everything with `nice` and `--max-jobs 6 --cores 4` so
   the host stays usable). Record run counts and failure rate. A
   no-reproduction result after a serious attempt (≥20 contended runs) is
   an honest outcome — record it and downgrade the item with the evidence.
2. **Root cause with mechanism proof** if it reproduces: journal ordering,
   unit dependency graph, and the exact interleaving. No fix without the
   proof in the LOG.
3. **Fix**: if it is the suspected oneshot-ordering race, the fix likely
   lives in the generated unit wiring (ordering/binding on the pod netns
   unit or readiness signaling from it). Keep CIP-86 semantics unchanged;
   no new manifest surface. If the honest fix needs a design change,
   stop and report instead of improvising.
4. **Regression protection**: extend the focused netns scenario (or add a
   contended variant) only if it can assert the mechanism deterministically
   — no flaky load tests in CI. Otherwise the LOG receipt is the record.
5. Ledger currency: update the docs/open-questions.md entry with the
   outcome in the same track.

FENCE: your domain is the netns/pod wiring in crates/cix-run (and its unit
generation), `nix/scenarios/` netns scenarios, and the open-questions
entry. Do not touch corpus/, docs/corpus*, docs/migrate.md, cips/, or
health/liveness code (track/adapterlive runs concurrently there). If the
fix genuinely requires crossing into shared unit-generation code that
adapterlive also touches, note it in the LOG and proceed carefully — the
orchestrator resolves overlap at merge.

## Gate

Standard agent tier (fmt, examples fmt, warning-denied clippy, full
workspace tests, tour regen+drift) plus the FOCUSED netns VM scenarios.
Announce in your LOG before starting any long contended-load experiment,
and keep each bounded as above. Receipts are synchronous exit statuses.
