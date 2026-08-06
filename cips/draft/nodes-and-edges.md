# nodes-and-edges — argv-first steps, explicit dataflow, the shell in a heredoc

Status: **draft** (2026-08-05; the bundled language round from Mathijs's
shell/ENV design sessions. Supersedes draft/shell-directive.md.)

## 1. The problem, and the principle

RUN and FETCH are nodes in a graph that cix manages — keyed, memoized,
traced, replayed. Dataflow between nodes (versions into URLs, values
into environments, hashes into pins) is the *edges* of that graph, and
edges are exactly cix's scope. Today the language outsources its edges
to bash: every RUN/FETCH is silently `bash -c`, and ENV plays a double
role — sometimes a genuine process-environment contract, usually just
the only variable-like thing in reach. Docker taught this shape: a
Dockerfile *reads* as one bash script but is N one-shot shells
stitched together by Docker's env dict; the illusion works only
because bash variables and the env dict are both flat strings (which
is also why "a Dockerfile in pwsh" never works). Measured in our
corpus (32 builder ENVs): ~20 are pure text-substitution variables
(versions/arches/shas/urls, referenced as `$NAME` in step text) and
~12 are real tool-environment knobs (`npm_config_*`, `GOMODCACHE`,
`NODE_OPTIONS`) referenced **zero** times in text — read only from the
environment. Upstream mirrors it: 11 Dockerfiles use ENV vs 4 ARG,
with `ENV VERSION=…` + `ENV URL=…${VERSION}…` as the canonical
variables-through-ENV idiom. Two disjoint populations, one conflated
directive, one invisible interpreter.

## 2. Prior work

- **Docker**: exec-form `RUN ["tar","-xf","x"]` is the bash-less RUN
  nobody uses — JSON ergonomics killed it, not the idea. `SHELL` is
  mutable mid-file state, widely regarded as a wart. Heredoc RUN with
  shebang exists since BuildKit.
- **systemd**: `ExecStart=` is argv; wanting a shell means writing
  `/bin/sh -c` visibly. **GitHub Actions**: `shell:` is per-step, not
  global — the sane version of SHELL.
- **Contracts**: `-c` is POSIX-standard for sh-compatible shells (fish
  kept it; sshd/make/cron/libc `system(3)` rely on it) but mere
  convention for interpreters; the kernel's only interpreter contract
  is the shebang — "accept a filename".
- **Modern shells all un-conflate the environment**: pwsh `$Env:VAR`,
  nushell `$env.FOO`, elvish `$E:VAR`, YSH's explicit `ENV` dict; the
  env boundary is strings-only everywhere (bash cannot export arrays;
  zsh needed tied vars). fish separates scope from export as an
  orthogonal flag and has native list variables that expand to argv
  elements without quoting hell.
- **Expansion staging**: Homer, it-tools, and Mailpit each first failed because
  `RUN`/`FETCH` needed an explicit imported Bash even when the visible command
  was a package manager. This is the third corpus datapoint for making the
  interpreter edge explicit rather than inherited from an image.
- **Session-shells** (state carried between steps): analyzed and
  rejected below.

## 3. Recommendation

The boundary is the design: outside a heredoc everything is cix
(argv, declared edges, interpolation); inside it everything is the
named interpreter.

1. **`RUN <argv>` / `FETCH <argv> …` are argv-first**: direct exec, no
   shell, no expansion. `$X` in argv-mode is a parse-time teaching
   error: "declare `LET X` and write `${X}`, or use a heredoc".
   **One command per node is the canon**: Docker's `&&`-chain exists
   to minimize layers — a cost cix does not have. Chains fuse nodes,
   losing memo granularity (editing step b re-runs step a), and are
   poor-man's fail-fast that separate nodes provide natively (cix
   checks every node's exit). Migration decomposes chains into
   successive RUNs; the FETCH normalization tails
   (`&& chmod 644 && touch -d @0`) are a missing feature, not a
   heredoc case — see the NORMALIZE open question.
2. **Heredoc is THE structural form** (Mathijs, 2026-08-05: complex
   RUNs go through heredoc, full stop — no one-liner shell fence).
   `RUN <interp> <<EOF … EOF`: the body is written to a file and the
   interpreter invoked with the filename — the shebang-grade kernel
   contract. Consequences: no `-c` reliance anywhere (works for any
   interpreter, node included); no fence-marker syntax exists or
   needs bikeshedding; no interpreter allowlist — any executable from
   the IMPORTed/locked closure that accepts a filename qualifies
   (bash/fish/nushell/pwsh are the documented examples). A one-line
   pipe costs three lines — deliberate friction that pushes toward
   decomposition, consistent with the chain ban.
3. **Node attachments are adjacency-bound clauses** — one grammatical
   shape for everything that hangs off a node: `WITH` (environment,
   below), and `EXPECT` migrates to the same position for heredoc
   FETCHes, so the phpmyadmin-class GPG pipeline has a home. Binding
   is by ADJACENCY, not indentation (Mathijs, 2026-08-05): a clause
   line must immediately follow its step line or another clause of
   the same step — elsewhere it is a parse error. Indentation stays
   what it already is in this language: cosmetic, canonicalized by
   `cix fmt` (+2 under the node), enforced through the fmt/drift
   gate — no significant-whitespace regime enters the parser, no
   clash with the ITEM/SERVICE/BUILDER phase convention. For heredoc
   nodes, clauses follow the terminator (`EOF` then the WITH/EXPECT
   lines). Braces were considered and dropped (first `{}` block in a
   keyword-driven grammar, buys nothing over adjacency+fmt); Docker's
   `######` phase fences may be blessed as an fmt-preserved comment
   convention, never as syntax. Simple node = one line; complex node
   = line + clauses + optional heredoc body — the systemd/GHA block
   shape, graduated instead of mandatory.
4. **Binders: LET in, builder-ENV out** (Mathijs, 2026-08-05:
   explicit per-node binding is "obviously correct" — and the ban is
   scoped: ENV remains the LEAF-phase (SERVICE/APP) runtime-contract
   directive with its CIP-96/100 vocabulary — `ENV PORT=8080`,
   `ENV API_TOKEN required`, bare `ENV NAME` optional. What dies is
   ENV as builder-scope broadcast state):
   - `LET NAME = value` — a text edge: file-local, interpolates as
     bare `${NAME}` in argv/directive/FILE positions (D32's bare-name
     objection targeted *ambient* bindings; a declaration three lines
     up is not ambient). **Never exported** to the environment.
   - `ARG` — a LET whose value is selected from a declared closed
     matrix (per draft/build-args.md); same interpolation.
   - **Builder-scope `ENV` is banned.** Environment is bound per
     node, as indented `WITH` clauses attached to the step (the
     GHA-`env:`/systemd-`Environment=`/nix-attr shape, in Cixfile's
     existing indentation idiom):

         RUN pnpm install --frozen-lockfile
           WITH COREPACK_ENABLE_PROJECT_SPEC=0
           WITH NODE_OPTIONS=--max-old-space-size=8192

     `WITH NAME=value` binds a string into that node's environment;
     bare `WITH NAME` pulls the value from the LET of that name (the
     explicit LET→env bridge, one token per consuming node). The
     assignment/bare shapes mean environment; future node attachments
     spell their kind (`WITH UNSAFE IGNORE <path>` — recorded
     direction, renamed from the earlier `WITH CACHE` sketch (Mathijs,
     2026-08-06): D71's underlay already owns warm persistence, so the
     clause's only real semantics is EVIDENCE EXCLUSION — the path
     leaves the read set, the seal/pins, and all keys; `--cold` starts
     it empty like everything else and the output seal still verifies
     what remains claimed. The UNSAFE prefix is the Rust-`unsafe`
     convention: an author-carried invariant cix cannot check, and a
     diagnostic names the waived evidence at every use. CACHE as a
     word stays reserved for D71's recorded retreat dial. The
     npm_config_cache/GOMODCACHE knob class dissolves the same way it
     would have under the old name). Env edges are per-node declared text, so keying is
     per-node by construction — no shared env state exists. Inside a
     heredoc, `export` and `VAR=x cmd` remain available as
     node-INTERNAL state (the interpreter's interior, not an edge).
     Leaf-phase `ENV` (SERVICE/APP — the runtime contract,
     CIP-96/100 forms) is untouched: the service is the node and
     declares its environment there.
5. **What the heredoc does and does not promise**: inside a heredoc, the environment is visible and expandable — that leak is
   real, unavoidable (tools must read their knobs), and scoped: it
   exists only where the author explicitly named an interpreter. The
   converse is enforced: LETs never enter the environment, so the two
   kinds cannot erode into each other.
6. **No SHELL directive** (supersedes draft/shell-directive.md):
   per-line explicitness beats block or file state, per the
   systemd/GHA lesson and Docker's SHELL wart.

**Edge granularity** — with the ENV ban this section simplifies: there
is no shared environment state left to key coarsely. LET edges appear
per-use in resolved text; WITH edges are per-node declared text; both
key precisely by construction. The unobservability analysis below is
retained as the reason a *broadcast* ENV could never have been traced
into precision — the ban removes the problem rather than approximating
it.

**Edge granularity — why LET keys precisely and ENV honestly-coarsely**
(Mathijs's follow-up: "moet je env vars dan niet ook traceren?"). File
edges are precise because reads are syscalls strace can see. Env reads
are structurally unobservable: the block is handed wholesale at
`execve`, `getenv` is a memory access, and the tools ENV exists for
read wholesale anyway (npm prefix-scans all of environ) — their honest
read-set IS the whole environment. Today every builder ENV is a
chain-prefix step, so any change invalidates all subsequent steps.
The triad fixes granularity exactly where it can be fixed: the
frequently-changing population (versions/hashes) becomes LETs, whose
edges appear per-use in resolved step text and therefore key
precisely; the residual ENV population is rarely-changing and
wholesale-read, where prefix-coarse keying states the truth rather
than approximating it (the CIP-102 pattern: precision where
observable, declared coarseness where not). A later refinement exists
if dogfood demands it — per-step declared env subsets, keying each
step on only the ENVs it receives — recorded as direction, not built.

**Considered and rejected — runtime session-shells** (a Cixfile "in"
nushell/fish/pwsh with carried variable state): state blobs are opaque
to tracing, so memos degrade to whole-blob keying (Docker-layer
semantics return); cold replay would need canonical serialization plus
a second volatility apparatus for `$RANDOM`/`$PID`-class
nondeterminism; carried vars are a pin-evasion channel around FETCH;
and principally, session state is an edge the graph manager cannot
see. The workspace (D71 end-state carry) IS the session; state that
matters lives in the filesystem where tracing, normalization, and
pins observe it. The charm survives as **frontends**: a nu/fish
dialect can compile to stateless steps (statically resolvable vars
lower to LETs; cross-step dynamic values materialize through files,
making the edge traced) — generators, never runtime. No language slot
is reserved; frontends emit Cixfiles.

Migration: mechanical corpus/examples/tour sweep — each RUN/FETCH
either argv-izes (chains decomposed per the canon) or becomes a
heredoc; the ~20
variable-ENVs become LETs (their sha-values largely dissolve into
lock pins); migrate.md teaches the boundary rule in one sentence.

## 4. Open questions — resolutions (Mathijs review, 2026-08-06)

- **Heredoc interpreter**: mandatory (`RUN bash <<EOF`) — a bare
  `RUN <<EOF` has no interpreter to name; confirm.
- **EXPECT-as-clause migration** — RESOLVED: a formatting concern, not
  a grammar break. Clauses (WITH and EXPECT alike) may be written
  inline on the node line; `cix fmt` canonicalizes to the indented
  clause position (multi-line under the node). The migration sweep is
  therefore fmt-driven — no parse-validity cliff, no flag day for
  inline suffixes.
- **FETCH normalization tails** — RESOLVED (direction, pending
  Mathijs's confirm on the mechanism): no `NORMALIZE` directive.
  Normalize always, at the fingerprint/seal layer, by adopting NAR
  semantics. Facts grounding this: a nix NAR records only file type,
  content, the executable bit, and symlink targets — mtime, ownership,
  and non-exec mode bits do not exist in it, and store import
  normalizes them away (mtime→1, perms→444/555). Our `narHash`/
  `snapshotNarHash` pins already have this invariance, but the
  per-path pin hash (`read_hash`, cix-build/src/trace.rs) folds the
  FULL `st_mode` bytes in, and lock-side fingerprints carry
  dev/inode/mtime — both strictly broader than NAR, which is exactly
  why `&& chmod 644 && touch -d @0` tails exist in the corpus.
  Aligning every fingerprint to NAR semantics (content + exec bit +
  symlink target, nothing else) makes the tails vacuous; migration
  deletes them. This is the same repair as the fmt-key-neutrality
  draft's ntfy exhibit (sourceHash churn from dev/inode fingerprints
  with identical content) — one keying-invariance fix serves both.
- **LET list values** — RESOLVED (syntax reserved, feature still out
  of v1): unify enumeration syntax with ARG so the grammar has ONE
  list form. `LET FLAGS = [a, b, c]` binds a list value expanding
  fish-style into argv elements (the growth path env structurally
  cannot follow); ARG's closed matrix reuses the same literal —
  `ARG VERSION from [1.24.2, 1.25.1]` — replacing the earlier
  `1.24.2 | 1.25.1` alternation sketch. ARG = select ONE from a
  declared list; LET-list = bind ALL. Prior work: fish lists expand
  to argv elements natively; nix/nushell lists are the same shape;
  nothing in prior work couples one-of and all-of into one construct,
  so keeping them as two keywords over one literal is the consistent
  cut. v1 ships ARG-with-list-matrix; LET-lists follow when dogfood
  demands.
- **Effort/staging** — RESOLVED: nodes-and-edges + phase-blocks +
  build-args land together as ONE language epoch (one corpus sweep,
  one migrate.md rewrite); the LET/ARG/ENV triad is part of it.
