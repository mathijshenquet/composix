# nodes-and-edges — argv-first steps, explicit dataflow, the shell behind a fence

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
- **Session-shells** (state carried between steps): analyzed and
  rejected below.

## 3. Recommendation

The fence is the design: outside it everything is cix (argv, declared
edges, interpolation); inside it everything is the named interpreter.

1. **`RUN <argv>` / `FETCH <argv> …` are argv-first**: direct exec, no
   shell, no expansion. `$X` in argv-mode is a parse-time teaching
   error: "declare `LET X` and write `${X}`, or use a fence".
   **One command per node is the canon**: Docker's `&&`-chain exists
   to minimize layers — a cost cix does not have. Chains fuse nodes,
   losing memo granularity (editing step b re-runs step a), and are
   poor-man's fail-fast that separate nodes provide natively (cix
   checks every node's exit). Migration decomposes chains into
   successive RUNs; the FETCH normalization tails
   (`&& chmod 644 && touch -d @0`) are a missing feature, not a
   fence case — see the NORMALIZE open question.
2. **The fence is the rare form, not the workhorse**:
   `RUN bash $ <text>` desugars to argv `[interp, "-c", text]`
   (`-c` is POSIX-standard for shells; the `$` reads as a prompt and
   documents the interpreter on the line). Legitimate uses shrink to
   pipes, redirects, and genuinely shell-shaped one-liners; anything
   longer belongs in a heredoc. The interpreter must resolve into the
   IMPORTed/locked closure.
3. **`RUN <interp> <<EOF … EOF`** — heredoc form: body written to a
   file, interpreter invoked with the filename (the shebang-grade
   kernel contract; works for any interpreter, no `-c` assumption).
4. **Binders: LET in, ENV out** (Mathijs, 2026-08-05: explicit
   per-node binding is "obviously correct"):
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
     spell their kind (`WITH CACHE …` — recorded direction: cache
     mounts would dissolve the npm_config_cache/GOMODCACHE knob class
     entirely). Env edges are per-node declared text, so keying is
     per-node by construction — no shared env state exists. Inside a
     fence/heredoc, `export` and `VAR=x cmd` remain available as
     node-INTERNAL state (the interpreter's interior, not an edge).
     Service-side `ENV` (the runtime contract, CIP-96/100) is
     untouched: the service is the node and declares its environment.
5. **What the fence does and does not promise**: inside `bash $` or a
   heredoc, the environment is visible and expandable — that leak is
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
either argv-izes or gains its `bash $`/heredoc fence; the ~20
variable-ENVs become LETs (their sha-values largely dissolve into
lock pins); migrate.md teaches the fence rule in one sentence.

## 4. Open questions

- **Marker token**: `$` (prompt mnemonic) or an alternative; parse it
  as a standalone token between interpreter and text.
- **Heredoc interpreter**: mandatory (`RUN bash <<EOF`) or is a bare
  `RUN <<EOF` allowed with an implied… no — proposal says mandatory;
  confirm.
- **FETCH normalization tails**: the `&& chmod 644 && touch -d @0`
  idiom keeps forcing fences around single-command FETCHes — promote
  to FETCH options (`FETCH … NORMALIZE`) now or later?
- **LET list values** (later): fish-style list expansion into argv
  elements is the growth path that ENV structurally can never follow
  (strings-only env boundary) — out of v1, recorded as direction.
- **Effort/staging**: parser + executor + sweep is M-L; whether the
  LET/ARG/ENV triad lands together with argv-RUN or as two tracks.
