# track/famtags — D62 round one: declared names, JSON build output, selector, tag-only -t

Read AGENTS.md first. Authoritative: design.md **D62** (read it fully; it is the
contract). Scope: crates/cix-cixfile (+cix CLI surface, cix-index only for tag-call
sites), examples, docs (cixfile.md, migrate.md, docker.md rows, README.md example),
tour. This is D62's ROUND ONE: family/member land as plain slashed names in the
existing per-name tag tables — do NOT touch the index table schema (the
`tag → member-map` form and atomic multi-member publish ride the D46 work later).

1. **`NAMESPACE <name>` directive**: at most one, top-of-file region (with the
   FROMs). Name = optional host-qualified, schemeless (`cix.my-org.com/my-app` ok,
   `http://…` = migration-grade error saying scheme is transport, not identity).
   Not a binder: no `${…}` participation; ignored when the Cixfile is consumed as
   a source context by another build. NOT written into generated manifests or any
   built output — grep the codegen to prove nothing leaks. Required (parse error)
   when the file has >1 artifact block; optional otherwise.
2. **`cix build .` output**: no `-t` ⇒ print ONLY a JSON object
   `{"<member>": "<store path>", …}` on stdout (always this shape, also for a
   single member; human logs stay on stderr). Tags nothing.
3. **`cix build .#<member>`**: build only that artifact block's backward DAG
   slice (builders/FETCHes it transitively references — verify with a test that
   an unrelated builder does NOT execute). Prints the bare store path only.
   Selector + `-t` together = error ("a tag names the whole family").
   Unknown member = error listing members.
4. **`-t <tag>` semantics**: tag-only (a `:`, `/`, or full-ref form in -t =
   migration-grade error: "names moved into the Cixfile (D62): declare NAMESPACE/
   SERVICE names there; -t takes only tags"). Repeatable: each tag applied to
   every member as `<family>/<member>:<tag>` via the existing cix_index::tag
   (single-member family without NAMESPACE: `<member>:<tag>`, no slash).
   `--namespace <name>` overrides the declared namespace; error if it carries a
   scheme. No `-t` and no selector ⇒ point 2. Tag refs never default: any ref
   accepted by run/pull/inspect without an explicit `:tag` must error (check the
   existing ref parser; add the migration-grade ":latest is not a thing here"
   guidance when the ref looks like a docker-style untagged name).
5. **Ref grammar**: names may now contain single `/` segments
   (`[host/]family/member:tag`); host recognized docker-style (first segment
   containing a dot or port). Family/member/block names: existing block-name
   charset, no `/` or `:` inside segments. Run/pull/inspect must resolve slashed
   names through the existing index unchanged (they are plain names in tables —
   prove with a run-by-tag test).
6. **Sweep**: README.md quickstart (SERVICE my-nginx; `cix build . -t v1` ⇒
   `my-nginx:v1`), examples (proj1 gets a NAMESPACE — it is multi-artifact),
   docs, tour regen. The old tour tag flows (`-t tour-app:v1` style) migrate to
   declared names + tag-only -t.
7. Gate: `devenv shell -- cargo fmt --all --check`; warning-denied workspace
   all-target clippy; `cargo test --workspace`; tour regen + drift + determinism
   twice; `devenv shell -- nix build .#checks.x86_64-linux.vm-dogfood --no-link -L`.
   Exact repros + user-unit cleanup in crates/cix-cixfile/LOG.md. Commit on this
   branch when green.
