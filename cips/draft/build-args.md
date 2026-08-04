# build-args — lock-pinned ARG (parameterizable Cixfiles without ambient inputs)

Status: **draft** (2026-08-04; from Mathijs's dispositions review:
"wat is er tegen ARG? handig om dingen parameteriseerbaar te maken").

## 1. The problem

Docker's `ARG` lets one file build many variants (versions, feature
flags) from CLI input. The recorded refusal (D32-era) had one real
argument: an ambient CLI channel breaks "Cixfile+lock is the whole
truth" — a build would depend on values recorded nowhere, and replay/
audit (--cold, buildCixfile) could not reproduce it. The
parameterization convenience is nevertheless real; today's answers
(edit the file; twins via --file; compose parametrics for deploy time)
cover variants but not e.g. a CI matrix building N versions of one
Cixfile.

## 2. Prior work

Docker ARG: ambient, unrecorded, notoriously surprising scoping. Nix
flakes: function arguments with defaults, overridden explicitly and
visible in the caller. Our own precedent: `--update-lock` moves —
CLI-initiated changes become visible lock diffs.

## 3. Recommendation

ARG, but lock-pinned: `ARG NAME=default` declared in the prelude;
`cix build --arg NAME=value` overrides it AND records the resolved
value in `Cixfile.lock` (an override is a visible lock diff, exactly
like a lock move). `${NAME}` interpolates like any binder. Replay
(--cold, buildCixfile) reads the value from the lock, so file+lock
stays the complete truth. Scope deliberately narrow: plain strings
entering interpolation — no conditionals, no computation (the
D28/D69d boundary stands).

## 4. Open questions

- Does an --arg override REQUIRE --update-lock-style explicitness, or
  is writing the lock on any override acceptable?
- Interaction with `--file` twins: args per file (each lock records
  its own) — confirm no shared state.
- Does the gitea version-stamp corpus case become the acceptance test?
