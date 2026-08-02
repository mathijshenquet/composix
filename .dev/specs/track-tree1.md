# track/tree1 — CIP-85 leg 1: the group-node grammar

Read AGENTS.md first (focused agent gate; synchronous receipts). 
Authoritative: cips/accepted/0085-compose-tree.md (the tree grammar,
instance identity, ref/lock floors) with D41–D46 in docs/design.md as
the decision records (they win on conflict). Work in
`/home/mathijs/worktrees/composix/track-tree1` (herdr worktree) on
branch `track/tree1`. Keep `crates/cix-compose/LOG.md` current.
Nothing else is in flight.

SCOPE FENCE — leg 1 is the tree WITHOUT networking: the `network`
field ("pod") is NOT accepted this leg (schema-rejected with a
pointer to the netns track) — accepted-but-inert would be a lie.
Publish climbing beyond the host boundary is likewise out; existing
edges/listeners keep working within and across nesting levels where
mechanics allow, refusals are loud where they don't.

1. **Grammar** (compose schema v2): a compose file is a group-node —
   `children` map whose values are ref-nodes (`{"item": "name:tag"}`
   selecting a single-service item per D41) or inline nested
   group-nodes; `edges` at any group level; D46: no computation,
   publish-time expansion only. The flat v0 `services` map keeps
   parsing with a migration-grade deprecation pointing at `children`
   (D72 alpha rules apply — pick refusal vs acceptance-with-warning
   and record; lean refusal per house style).
2. **Instance identity = path in the tree** (D42): units
   `cix-<path…>-<svc>.service`, nested slices
   (`cix-<comp>.slice` > `cix-<comp>-<child>.slice`), one target per
   root composite, path-keyed role-dir roots and shared surfaces. Two
   instances of one artifact under different keys are fully disjoint —
   prove it in the scenario.
3. **Ref/lock on every floor** (D44): every child ref is `name:tag`;
   the root lock records the resolved pin per PATH (not per artifact);
   `--update-lock <path>` moves one subtree's pin. A nested group may
   itself be a tagged compose artifact ref (`{"compose":
   "name:tag"}`) — resolved, pinned, and its own lock ignored in
   favor of the root's (one lock per deployment root).
4. **Host root** (D42): `cix root` file — same group-node format in a
   mutable file + adjacent lock; `cix up <root-file>` activates the
   whole tree; day-two verbs operate as structured edits (`cix root
   add/remove <path> <ref>` minimal set this leg). Rollback stays
   per-root-composite via the existing profile machinery.
5. **Docs/ledgers**: docs/design.md "Building now" note; docker.md
   project-namespacing + compose rows; corpus rows that cited
   tree-node properties (replicas stays ⏳ — replicas are NOT this
   leg); CIP-85 changelog line marking leg 1 landed.
6. **Tests**: schema accept/reject (nesting, network rejection, v0
   deprecation); unit-naming/slice snapshot fixtures for a two-level
   tree; lock-per-path tests; new `nix/scenarios/tree.nix`: a nested
   composite (root with one inline child group + one child by ref)
   comes up, two instances of the same item under different paths
   stay disjoint (state dirs, units), `--update-lock` on one path
   restarts exactly that subtree, `cix root add`/`remove` round-trips.

If the scope proves bigger than one track mid-flight, make an honest
STOP with a proposed split rather than a shallow pass (AGENTS.md
scope rule).

Gate (agent side): fmt / examples fmt / warning-denied clippy /
workspace tests / tour regen + drift / focused: scenario-tree +
scenario-lifecycle + scenario-side-by-side. Full matrix runs at the
orchestrator gate. Commit on this branch when green.
