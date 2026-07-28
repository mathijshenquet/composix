# cix — local index tour

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

This five-minute tour covers local tags, serving a store, pulling from it, and running a service.

## Scenarios

- [Tagging a build](01-tagging.html) — Give an immutable Nix store path a memorable local name.

- [Moving a tag](02-moving.html) — Retag a name to point at a newer build.

- [Untagging](03-untagging.html) — Remove a local tag and its GC root.

- [Serving your store](04-serving.html) — Expose bare local tags over HTTP.

- [Pulling on another machine](05-pulling.html) — Adopt a qualified remote tag under a local name.

- [Tags move; pull follows](06-pull-follows.html) — Refresh a remote mirror after its publisher retags it.

- [Running a service](07-running-service.html) — Start and inspect a spec'd service in rootless development mode.
