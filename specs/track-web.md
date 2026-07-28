# Track: web — namespace claims + informative serve pages

Read `DESIGN.md` first: "Part 1 — index", subsection "Claims & web pages (D17, D18)", and
decisions D12, D17, D18. That is the contract; this file adds implementation constraints. Where
ambiguous, choose the boring option and note it in your LOG — do NOT expand scope.

## Ground rules

- Work log: append to `crates/cix-index/LOG.md` (timestamped entries, keep current).
- Territory: `crates/cix-index/` and (only if truly shared) `crates/cix-common/`. Do NOT touch
  `crates/cix-run/`, `crates/cix/src/main.rs`, `DESIGN.md`, `docs/`, or `crates/cix/tests/`
  (another agent works there in parallel).
- No new heavyweight dependencies. HTML is small enough to generate with a hand-rolled
  escape function — no template engine.
- Commit to your branch as you go, meaningful messages.
- All existing tests must keep passing. `cargo fmt --check`, `cargo clippy --workspace -- -D
  warnings`, `cargo test --workspace` green at the end.

## Deliverable 1: claims (D17)

1. Storage: `claims.json` in the cix state dir (`Store`), a sorted list of claim patterns.
2. Pattern semantics (exactly this, nothing more): a pattern is matched against the string
   `{root_url}/{name}`. Two forms:
   - exact: `cix.example.com/team/app` matches only that name;
   - prefix glob: trailing `*` matches any suffix, e.g. `cix.example.com/*` or
     `example.com/team/*`. `*` is only allowed at the end. Validate patterns on input: the part
     before the glob must be a valid `host[:port]` optionally followed by `/` + name segments.
3. CLI (add to the existing `Command` enum in `crates/cix-index/src/cli.rs`):
   - `cix claim <pattern>` — validate, store, idempotent.
   - `cix unclaim <pattern>` — remove exact pattern; error if absent.
   - `cix claims` — list, one per line.
4. Enforcement:
   - `cix tag` where the target ref is qualified: require a covering claim, else error:
     `not claimed: <root_url>/<name> — if you control this namespace, run: cix claim <root_url>/*`.
   - `cix serve <root_url>`: require a claim covering `<root_url>/*` (or any claim whose host
     part equals the root_url), same error shape.
   - Implicit claim: any root_url whose host is `localhost` or `127.0.0.1` (any port) is always
     claimed. This must keep the existing integration tests passing unchanged.
   - `cix pull` (including `--as` with a qualified alias): NO claim check.
5. Tests: unit tests for pattern validation + matching (exact, glob, host-only, port
   sensitivity, invalid patterns like `a*b/x`, `*`, `foo/*` with non-host first segment);
   enforcement tests for tag and serve (error message contains the hint); localhost exemption
   test; pull-is-exempt test.
6. Update `crates/cix-index/demo.sh`: add the claim step before qualified tagging (use a
   non-localhost fake host for the claim demonstration only if it doesn't require networking;
   otherwise demonstrate claims with `cix claims` listing and keep the flow on localhost).

## Deliverable 2: informative pages (D18)

On the existing serve HTTP server:

1. `GET /` → HTML: page title = root_url, list of served names, each linking to `/{name}`.
2. `GET /{name}` (any name path not under `/v1/` or `/store/`) → HTML page:
   - heading `{root_url}/{name}`;
   - table of tags: tag, systems, store path, narHash, age;
   - a copy-pastable snippet: `cix pull {root_url}/{name}:{tag}`;
   - if the current system's store path exists locally and contains `cix-spec.json`, a summary
     section: service names, their declared ports and env var names. Parse it leniently as
     `serde_json::Value` — do NOT depend on `cix-run` types; if parsing fails, omit the section
     silently.
   - unknown name → 404 with a small HTML body.
3. Content negotiation: when the `Accept` header contains `application/json`, `/` behaves like
   `/v1/names` and `/{name}` like `/v1/tags/{name}`. The `/v1/` and `/store/` routes are
   unchanged.
4. HTML: valid, minimal, self-contained (one small inline `<style>`, no external assets, no JS).
   Escape all interpolated strings.
5. Tests: extend the existing serve integration test — fetch `/` and `/{name}`, assert 200,
   `text/html` content type, and that the tag and a pull snippet appear; assert the JSON
   negotiation path; assert 404 for unknown names.

## Done criteria

fmt/clippy/tests green; demo.sh runs; LOG.md has a final summary entry listing any deviations
from DESIGN.md and open questions.
