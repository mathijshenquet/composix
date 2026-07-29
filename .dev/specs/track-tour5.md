# Task: tour coverage pass — Cixfile build scenario (+ listeners if feasible)

The tour (crates/cix/tests/tour.rs → docs/tour/) covers index flows and a basic
`cix run --user`. It does not cover `cix build` (Cixfile) or spec-v3 listeners. Extend it,
same conventions as always (Doc builder, normalization, drift + determinism, unique ports,
shown command = executed command).

Territory: `crates/cix/tests/`, `docs/tour/` (generator only), `examples/` READ-ONLY.
COMMIT AS YOU GO.

1. **"Building from a Cixfile" scenario** (after the existing run scenario): a minimal
   Cixfile written by the harness into a temp dir (NOT one of the examples — keep the tour
   self-contained): one FILE, one SERVICE with EXEC of a tiny script via SCRIPT, no PKG if
   possible (verify whether zero-PKG Cixfiles build — if codegen still needs the nixpkgs pin,
   commit a lock the harness copies in place so generation is deterministic and cold-network
   is only needed once on a fresh store; note the behavior in the scenario prose). Show:
   `cix build . -t tour-app:v1` (normalize the store path), the generated `cix-spec.json`
   (`cat`), `cix ls` showing the tag. Prose: determinism via the lock, the spec as the
   contract.
2. **Listeners scenario — feasibility first**: probe whether transient `.socket` units work
   against the USER manager (the tour cannot use root). If yes: a scenario running the
   listenfds-style fixture with `cix run --user -p http=127.0.0.1:<port> --detach`, curl,
   stop. If no: skip the scenario and record precisely why in `crates/cix/LOG.md` (that
   finding feeds the design).
3. Update `docs/tour/index.md`'s intro line to describe the enlarged coverage; renumber
   cleanly.

Gate: full workspace test suite green ×3 (drift + determinism over all pages); committed;
clean status; LOG updated.
