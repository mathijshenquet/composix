# track/fhsspike — patchelf spike for the FHS-interpreter gap (evidence only)

Read AGENTS.md first, then cips/draft/fhs-interpreter.md — this track runs
its §3 spike. **This is a spike: the branch is evidence, not a merge
candidate.** Nothing here lands on main; the deliverable is your LOG
report, which feeds a rewritten CIP with multiple concretely worked-out
syntax proposals (the orchestrator writes that from your evidence).

Work in the herdr worktree on branch `track/fhsspike`. Keep
`corpus/migrate/LOG.md` current in-worktree under a dated
`track/fhsspike` heading (it will be lifted into the report, not merged).

## The question

Can an explicit patchelf recipe inside a builder — using ONLY existing
language surface (`IMPORT ${pkgs.patchelf}` etc., offline `RUN`) — make
directus's downloaded native binary (the sass-embedded FHS-loader case,
the corpus's recorded blocker) build and ideally run? And what exactly
does the recipe require, so a language/teaching decision can be made on
evidence?

## Method

1. `bash corpus/migrate/fetch.sh directus`; reproduce the current failure
   with the checked-in Cixfile (synchronous receipt of the exact error).
2. Inventory every downloaded ELF in the dependency tree with an absolute
   FHS interpreter (scan the pnpm store/node_modules after fetch): names,
   interpreters, DT_NEEDED sets.
3. Extend the directus Cixfile (in this worktree — it never merges) with
   patchelf steps: `--set-interpreter` from the locked universe's glibc,
   `--set-rpath` from the packages whose libraries the DT_NEEDED sets
   demand. Iterate honestly; record every wall (missing libs, version
   mismatches, re-download/re-patch at runtime, pnpm integrity checks
   rejecting patched files, …).
4. If the build goes green: run `./check.sh cix` for the runtime probe,
   and record whether the item's closure now includes the patched-in
   store references (the lock/closure facts).
5. Bound everything; no VM scenarios needed.

## Report (the actual deliverable, final LOG entry)

- Verdict: expressible-today / expressible-with-pain / blocked, with the
  receipts.
- The exact minimal recipe (or the wall that stopped it).
- Ecosystem generality notes: what of this is directus/pnpm-specific vs
  generic (prebuilds, integrity manifests, runtime re-downloads).
- Raw material for syntax proposals: given the mechanics you saw, sketch
  2–4 candidate shapes (e.g. taught RUN pattern; a dedicated builder
  directive; an IMPORT-adjacent fixup declaration) with one honest
  paragraph each on what the mechanics demand of them. Do NOT edit the
  draft CIP — sketches belong in the LOG report.
