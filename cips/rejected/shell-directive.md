# shell-directive — the RUN/FETCH interpreter as a declared dependency (CIP-light)

Status: **superseded** by draft/nodes-and-edges.md (2026-08-05: the SHELL-directive shape imports Docker's mutable-state wart; the argv-first round replaced it with per-line fences) (2026-08-05; from Mathijs: "waarom zo hard
coupelen aan bash? misschien wil ik mijn Cixfiles in fish of zsh").

**Problem.** RUN/FETCH execute through bash today, but the coupling is
thinner than it looks. Principled: the interpreter must be a declared,
locked dependency (never ambient host shell), and the Dockerfile-bridge
default should match Docker's shell muscle memory. Incidental: two code
sites hardcode bash — `find_shell` looks up `bin/bash` in the IMPORTs,
and the env prelude emits bash `export` syntax (likely redundant next
to the `--setenv` process-environment injection, which is
shell-agnostic). Nothing else cares which interpreter evaluates the
step.

**Prior work.** Dockerfile has `SHELL ["executable", "params"]` — the
bridge itself already names this surface. Nix derivations take an
arbitrary builder executable; bash is stdenv's default, not a rule.

**Proposal.**
- `SHELL <interpolated-path> [args…]` as a builder-scope directive
  (Docker's semantics: applies to subsequent RUN/FETCH in that
  builder). Example: `SHELL ${pkgs.fish}/bin/fish -c`. The path must
  resolve into the IMPORTed/locked closure — ambient paths refused.
- Default unchanged: bash from IMPORTs, exactly today's behavior; the
  corpus and migrate.md stay bash-canon (teaching one shell is a
  feature).
- The shell identity + argv join the chain key: identical step text
  under a different interpreter is a different step.
- Drop the bash `export` prelude in favor of the existing `--setenv`
  injection (verify equivalence first — if the prelude covers a real
  case setenv does not, keep both and document why).

**Open question.** Consumer reality: is this dogfood-demanded beyond
preference? If no second consumer appears it can sit adopted-unbuilt
(⏳-style) without cost — the decision would then mainly bless the
principle (interpreter = declared dependency) and the key semantics.

**Effort.** S/M — directive parsing + find_shell generalization + key
inclusion + one scenario; the prelude cleanup is the only subtle part.
