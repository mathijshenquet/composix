# composix — new-user guide

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

Composix is a nix-native Docker analogue. Images become immutable Nix store items, and containers become hardened systemd units. Dockerfiles become Cixfiles that declare exactly what enters an item and what its process may use.

Start at [Chapter 1](01-hello-composix.html) and follow the guide in order; every shown command is executed by the tour harness.

## Chapters

- [Chapter 1: Hello, composix](01-hello-composix.html) — Build your first canonical service item and probe the rootless boundary.

- [Chapter 2: The Cixfile language](02-cixfile-language.html) — Learn binders, assembly, runtime declarations, and the directive vocabulary.

- [Chapter 3: Building: BUILDERs, FETCH, and the lock](03-building.html) — Pin network inputs, reuse audited work, repair an FHS binary, and build proj1.

- [Chapter 4: Naming and distribution](04-naming-distribution.html) — Tag immutable items, manage families, serve a cache, and follow a moving ref.

- [Chapter 5: Running: the runtime contract](05-runtime-contract.html) — Inspect health and observability, debug by tag, and schedule an APP.

- [Chapter 6: Compose](06-compose.html) — Connect two items with Unix and shared-dir edges, then inspect lifecycle boundaries.

- [Chapter 7: The dev loop and coming from Docker](07-dev-loop-docker.html) — Watch artifact rebuilds, keep translation twins, and continue into the migration corpus.
