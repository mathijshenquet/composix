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
   error: "declare `LET X` and write `${X}`, or wrap the line in
   `bash $ …`".
2. **`RUN bash $ <text>`** — the explicit-shell fence: desugars to
   argv `[interp, "-c", text]`. `-c` is POSIX-safe for any shell; the
   `$` reads as a prompt and documents the interpreter on the line.
   The interpreter must resolve into the IMPORTed/locked closure.
3. **`RUN <interp> <<EOF … EOF`** — heredoc form: body written to a
   file, interpreter invoked with the filename (the shebang-grade
   kernel contract; works for any interpreter, no `-c` assumption).
4. **Binder triad** (one namespace, three declaration kinds):
   - `LET NAME = value` — a text edge: file-local, interpolates as
     bare `${NAME}` in argv/directive/FILE positions (D32's bare-name
     objection targeted *ambient* bindings; a declaration three lines
     up is not ambient). **Never exported** to the environment.
   - `ARG` — a LET whose value is selected from a declared closed
     matrix (per draft/build-args.md); same interpolation.
   - `ENV NAME=value` — an environment edge: string into the process
     env, keyed as a step. Rarely referenced in text (the corpus's
     12-knob population); inside a fence it is `$NAME`, because the
     fence is honestly the interpreter's world.
   - Bridging is explicit dataflow: `ENV NAME=${VERSION}` — the zsh
     tied-var, spelled as a visible line.
5. **What the fence does and does not promise**: inside `bash $` or a
   heredoc, the environment is visible and expandable — that leak is
   real, unavoidable (tools must read their knobs), and scoped: it
   exists only where the author explicitly named an interpreter. The
   converse is enforced: LETs never enter the environment, so the two
   kinds cannot erode into each other.
6. **No SHELL directive** (supersedes draft/shell-directive.md):
   per-line explicitness beats block or file state, per the
   systemd/GHA lesson and Docker's SHELL wart.

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
