# cix — local index tour

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

This five-minute tour covers local tags, serving and pulling a store, building from a Cixfile, and running rootless services with or without socket activation.

## Scenarios

- [Tagging a build](01-tagging.html) — Give an immutable Nix store path a memorable local name.

- [Moving a tag](02-moving.html) — Retag a name to point at a newer build.

- [Untagging](03-untagging.html) — Remove a local tag and its GC root.

- [Serving your store](04-serving.html) — Expose bare local tags over HTTP.

- [Pulling on another machine](05-pulling.html) — Adopt a qualified remote tag under a local name.

- [Tags move; pull follows](06-pull-follows.html) — Refresh a remote mirror after its publisher retags it.

- [Running a service](07-running-service.html) — Start and inspect a spec'd service in rootless development mode.

- [Building from a Cixfile](08-building-cixfile.html) — Build, inspect, and tag a self-contained Cixfile item.

- [Running with a listener](09-running-listener.html) — Serve through a systemd-activated socket in rootless development mode.

- [Composing services](10-composing-services.html) — Validate and dry-diff a tracked compose service without root.

- [Debugging a service](11-debugging-service.html) — Run a deterministic command in a fresh service sandbox.

- [Inspecting artifacts](12-inspecting.html) — Read a tag's index entry and parsed manifest as stable JSON.
