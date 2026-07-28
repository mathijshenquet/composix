# Track: item-mount — env de-typing (D21) + stable /item mount (D22)

Read `docs/design.md`: amendments D21 and D22 under "Spec v2" are the contract, plus D13
(degraded --user). Where ambiguous, choose boring and note it in LOG. Do NOT expand scope.

## Ground rules

- Work log: append `crates/cix-run/LOG.md`. Territory: `crates/cix-run/`, `examples/`.
- Sudo available for system-mode verification; clean up all units, always.
- COMMIT AS YOU GO; done gate includes clean `git status --short`.

## Deliverable 1: env de-typing (D21)

1. Schema: env entries are `{default?, required?, secret?}`, all string values. The `type`
   field remains ACCEPTED AND IGNORED (deprecated — rustdoc note; do not error on it) so
   existing specs/fixtures keep parsing. Update validation: a var referenced by a `ports`
   entry must have a default (if any) and overrides that parse as a port 1–65535; the <1024
   capability logic keys off the *resolved* value as before.
2. Update examples' specs and test fixtures to the de-typed form (drop `type` from JSON).
3. Validation-error tests: ports referencing undeclared var; non-port-parsing default on a
   ports-referenced var; `-e` override of a ports-referenced var that doesn't parse.

## Deliverable 2: /item (D22)

1. System mode: every generated unit binds the resolved store item read-only at `/item`
   (`BindReadOnlyPaths=<store-path>:/item`) and sets `Environment=CIX_ITEM=/item`.
2. Verify empirically that `ExecStart=/item/bin/...` resolves inside the unit's namespace
   (mount ns is set up before exec) — if it does, also accept `/item/…` paths in
   `exec`/`setup` argv validation; if it does not, record why in LOG and keep exec
   store-path-based (D22's file-content story is unaffected either way).
3. Degraded `--user` mode: no bind possible — set `CIX_ITEM=<real store path>` and extend the
   existing degradation warning listing `/item` as unavailable.
4. Golden fixtures updated; a unit test asserting the bind + env lines; sudo demo re-run for
   both examples (they need no /item changes themselves — the Cixfile track will use it).

## Done gate

fmt/clippy/tests green; both sudo demos pass; no leftover units; committed; LOG summary.
