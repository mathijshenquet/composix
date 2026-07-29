# Track: composeformats — prototype the compose surface language (design doc, no code)

Deliverable: `docs/compose-formats.md` — the same compose scenarios rendered in candidate
formats, with honest pros/cons and a recommendation. This is the "serious prototyping" that
D3 demanded before choosing the compose language. No code; the document is the product.

Read first, this is your semantic contract: `docs/design.md` — D9 (resolve→lock→build→
activate), D20b (spec/item/compose boundary), D23–D27 (networking: composite netns, networks
as named objects, talks-to edges, caps tier), Part 3 parked notes, and cix-spec v2 (what the
spec already declares, so compose only carries operator decisions — overrides, wiring,
policy; never app contracts).

Study a REAL case: `a private fleet repo` (read-only!) — a frontend (node), a rust
workspace with a dashboard webserver, presumably nginx in front and state involved. Extract
its actual shape (services, ports, who-talks-to-whom, state) as scenario 2. Do not modify
anything there.

## Scenarios (render each in every candidate format)

1. **Minimal**: nginx + one app service, one published port, unix-socket wiring between them
   (D25), app state dir override.
2. **the private fleet repo, as found**: the real stack from the repo, incl. its actual ports/state,
   `talks-to` edges, one secret (e.g. an API key via credential), tag-tracking (`:latest`
   with hold/watch update policy per D9).
3. **Gnarly**: 6+ services, two networks (frontend/backend, D26) with per-service membership,
   a replicated worker (`scale: 3`), per-service resource limits, a service with a value-port
   collision forcing a remap decision, an adopted third-party item (postgres from cixpkgs)
   with operator env overrides, mixed update policy (pin one service, track another),
   cross-composite reference (this composite talks to another composite's published port).

## Candidate formats (at minimum these four)

- **TOML** — the boring candidate.
- **YAML** — the docker-compose incumbent (Mathijs detests it; include it honestly anyway as
  the baseline the world knows).
- **nix-lite** — nix syntax without fixpoints/overlays/functions-as-API: plain attrsets +
  string interpolation only; specify exactly which nix features are banned and how the file
  is evaluated (D4's "no one wants to write nix" tension must be addressed head-on).
- **A Cixfile-sibling DSL** — the directive style of the Cixfile extended to composition
  (SERVICE blocks referencing tags, NETWORK/TALKS-TO directives), so tag/build/compose share
  one syntax family.

You may add one more candidate if you believe in it (KDL, CUE, …) — argue it.

## Judging criteria (score each format; be adversarial, not polite)

References/reuse (can scenario-3's repeated limits be shared without a template engine?);
tag refs + lock interaction (how does `component:latest` read; where does the lock live);
diffability + code review; comments; schema validation story; how overrides of spec values
read; how networking edges (D26/D27) read; migration proximity for docker-compose users;
tooling cost for us (parser, formatter, LSP someday); footguns (YAML's norway problem etc. —
name them concretely per format).

## Output shape

`docs/compose-formats.md`: intro (what compose must express, per the design decisions),
scenario definitions in prose, then per-format: the three renderings in fenced blocks +
pros/cons; a comparison table; a clear recommendation with runner-up and what would change
your mind. Mark the whole doc as exploratory (a decision input, not a decision).

## Ground rules

Territory: `docs/compose-formats.md` ONLY (+ commits). COMMIT AS YOU GO; clean status at the
end. Final commit message body = your one-paragraph verdict.
