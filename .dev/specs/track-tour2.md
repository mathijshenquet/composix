# track/tour2 — the tour becomes a new-user guide (full restructure)

Read AGENTS.md first (gate convention; synchronous receipts). Work in
the herdr worktree on branch `track/tour2`. Keep `crates/cix/LOG.md`
current (dated track heading; commit it). The tour source of truth is
`crates/cix/tests/tour.rs` (CIP-90: tour from one truth); the .md pages
are generated. You are restructuring the harness so the generated tour
follows the blueprint below. The blueprint's chapter structure and
teaching goals are the orchestrator's design decisions — do not reorder
or drop chapters; DO exercise judgment inside each chapter (exact
examples, wording, assertions).

## Why

The current tour is internals-first: it opens with hand-built raw
manifests and tagging before a reader has ever seen a Cixfile build and
run. It also predates most of today's language: universal IMPORT,
store-aware COPY, udp ports, `--file` twins, FHS diagnostics; and it
never shows secrets, health, dirs, timers, watch, or logs. The tour
must become the guide a new user reads top to bottom.

## Voice rules

- Second person, present tense, one continuous story per chapter.
- Every command is real and executed by the harness; every output shown
  is asserted (existing discipline — keep it).
- Each chapter opens with two sentences: what you will build, and what
  you will understand afterwards.
- Docker familiarity may be assumed for contrast ("where Docker does X,
  cix does Y") but never required.
- A feature the harness cannot exercise honestly (needs a VM, external
  network at run time, root privileges the test lacks) is either cut or
  shown as clearly-labeled non-executed prose with a pointer to its VM
  scenario — never fake output.

## The blueprint (7 chapters)

1. **Hello, composix** — what composix is in three sentences
   (nix-native Docker analogue: images become store items, containers
   become systemd units, Dockerfiles become Cixfiles). Prereqs stated
   honestly. Then the five-minute win: the minimal SERVICE Cixfile
   (nginx pack-sample shape: IMPORT, COPY of index.html + config, bare
   START, PORT, role dirs), `cix build .`, run it, probe it, stop it.
   The reader runs their first service before any theory.
2. **The Cixfile language** — the model (backward-only graph, binders,
   no ambient names) taught through a growing example: IMPORT canon
   (bare argv, earlier-wins union), store-aware COPY (materialize vs
   link, show both), FILE and when it is a smell, ENV incl. `required`,
   role dirs at app-native paths, PORT incl. `udp:`, LISTENER in one
   paragraph, CLAIM egress/jit. Close with the directive table as
   reference.
3. **Building: BUILDERs, FETCH, and the lock** — FETCH+EXPECT and what
   the lock pins; offline RUN; warm workspaces and why re-runs are
   instant (read-set keying in one honest paragraph, not a lecture);
   `--update-lock` and `--cold` as the audit pair; the dev-env snapshot
   (no hand-wired toolchain paths); the CIP-95 FHS diagnostic shown FOR
   REAL: a step execs a downloaded FHS-linked binary without the libc
   import, the error text with its IMPORT hint appears, then the fix.
   Capstone: proj1 (one Rust workspace, two services, member-selective
   builds) — absorbed from old chapter 5.
4. **Naming and distribution** — compress old chapters 1–2: tag after
   build (names are operational, not build inputs), families,
   inspect/mv/rm, serve + pull + moving tags. Trim the raw-manifest
   hand-assembly to ONE short aside showing an item is just a store
   tree with a manifest — the demystification is worth keeping, the
   ceremony is not.
5. **Running: the runtime contract** — run by tag, debug, what closed
   root means for the process (read-only world, role dirs writable —
   show a failed write if the harness can), STATEDIR persistence across
   restarts, SECRET via credential file, READINESS/LIVENESS on a real
   HTTP service, APP + timer scheduling, `cix logs`/`ps`/`stats`. Use
   whatever the harness honestly executes; scenario pointers for the
   rest per the voice rules.
6. **Compose** — two services with a unix edge + shared dir,
   `compose check`/`up`/`diff`/`rollback`/`down`, `cix run` as unary
   compose, pods/netns in prose with scenario pointer, `logNamespace`.
   Old chapter 6's socket-activation content folds in here.
7. **The dev loop and coming from Docker** — `cix watch`, `--file` for
   faithful/dissolved twins side by side, then the bridge out:
   docs/migrate.md as the translation guide and the corpus browser as
   the worked-example gallery (link both).

index.md becomes the guide's front page: the three-sentence pitch, the
chapter list with one honest line each, "start at chapter 1".

## Mechanics

- Restructure `tour.rs` accordingly; keep the isolated-index execution
  model, the foreign-user-unit guard test, and drift checking exactly
  as they are. Page filenames renumber to the new order.
- Grep docs/ for links to old tour page names and fix them.
- This is a large rewrite — commit chapter by chapter in logical units
  so review can follow the story.

FENCE: track/cip94 (nix/lib + docs/nix-build.md) and corpus work run
concurrently — do not touch nix/, corpus/, docs/corpus*,
docs/migrate.md, docs/nix-build.md. Your domain:
crates/cix/tests/tour.rs, docs/tour/, doc links to tour pages, your LOG.

## Gate

Standard agent tier (fmt, examples fmt, warning-denied clippy, full
workspace tests, tour regen+drift — your own output must drift-check
clean). Bounded (`nice`, `--max-jobs 6 --cores 4`). Synchronous
receipts.
