# Track: web — serve refactor to D17 v2 / D18 v2 (bare-tag serving, negotiated URL space)

Read `DESIGN.md` first: decisions D17 (v2), D18 (v2), the "Part 1 — index" sections "HTTP
surface" and "The org workflow", and the ref-model block under D12. That is the contract; this
file adds implementation constraints. Where ambiguous, choose the boring option and note it in
your LOG — do NOT expand scope. The claims/publications designs you may find referenced in git
history are SUPERSEDED — do not implement them.

## Ground rules

- Work log: append to `crates/cix-index/LOG.md` (timestamped, keep current).
- Territory: `crates/cix-index/`, `crates/cix-common/` (only if truly shared). Do NOT touch
  `crates/cix-run/` or `crates/cix/src/`. Exception: if your (intended) output changes break the
  literate-tour drift test, regenerate via
  `cargo test --test tour -- --ignored generate_tour` and commit the updated `docs/tour.md` —
  never hand-edit it.
- No heavyweight dependencies; HTML via a hand-rolled escape helper, no template engine.
- Commit as you go. Done gate: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
  `cargo test --workspace` (all crates, including tour drift test) green; demo.sh runs.

## Deliverable 1: enforce the D17 invariant

1. `cix tag <installable> <qualified-ref>` → hard error:
   `qualified names denote remote state; tags are bare. To publish, tag on the box that serves
   (see DESIGN.md "The org workflow").`
2. `cix serve` loses its `root_url` positional argument entirely. It serves the **bare** tags of
   the local store only; qualified (mirror) tags are never served. All other flags stay
   (`--listen`, `--substituter`, `--with-store`, `--sign-key`).
3. `--with-store` maintains the binary cache for exactly the served (bare) closures.
4. The server does not know its own name: anything self-referential (pull snippets, advertised
   own-`/store/` substituter URL) is constructed per-request from the `Host` header, scheme from
   `X-Forwarded-Proto` when present else `http`.

## Deliverable 2: one negotiated URL space (D18 v2)

Replace the `/v1/…` routes entirely (delete them — nothing depends on them yet):

1. Routes = the name space: `GET /`, `GET /{name}`, `GET /{name}:{tag}` (name may contain `/`;
   the `:{tag}` suffix is after the last path segment). `/store/…` unchanged.
2. Negotiation: if the `Accept` header contains `application/vnd.cix+json` (any parameters) or
   `application/json`, or `?format=json` is given → JSON representation with content type
   `application/vnd.cix+json;version=1`. Otherwise → HTML (`?format=html` forces it). Always
   emit `Vary: Accept`.
3. JSON representations: `/` → `{"names": [...]}`; `/{name}` → `{"tags": {"<tag>": <entry>, …}}`;
   `/{name}:{tag}` → `<entry>`; unknown → 404 with a JSON body.
4. HTML representations (valid, minimal, one small inline `<style>`, no JS, escape everything):
   - `/`: served names, linked.
   - `/{name}`: heading `{host}/{name}`; tag table (tag, systems, store path, narHash, closure
     size via `nix path-info -S`, age); `cix pull {host}/{name}:{tag}` snippet; spec-summary
     section (service names, ports, env var names) when the current system's store path exists
     locally and contains a parseable `cix-spec.json` — parse leniently as `serde_json::Value`,
     omit the section silently on any failure. Do NOT import cix-run types.
   - `/{name}:{tag}`: that entry's detail (per-system outputs, narHash, drvPath if present,
     createdAt), plus `cix pull` and `cix run` snippets. This is the permalink page.
   - Unknown name/tag → 404 with a small HTML body.
5. Update the pull client: request `/{name}:{tag}` with `Accept: application/vnd.cix+json;version=1`.

## Deliverable 3: tests + demo

1. Update existing serve/pull integration tests to the new URL scheme (they should keep telling
   the same two-machine story).
2. New tests: conneg matrix (browser-ish Accept → HTML; vnd.cix and plain json Accept → JSON;
   `?format=` overrides; `Vary` header present); qualified-tag error (message content); mirrors
   are not served (pull a tag into a state dir without `--as`, serve from it, assert the mirror
   name is absent from `/`); 404 both representations; Host-header self-reference appears in a
   pull snippet.
3. Update `crates/cix-index/demo.sh` to the new flow (serve has no root_url; show a
   `curl` of the HTML page and of the JSON representation).

## Done criteria

Green gate as above; LOG.md final summary entry with deviations and open questions.
