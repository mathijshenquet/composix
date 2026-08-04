# cix — tour

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

This executable tour follows composix from naming and distribution through building, running, debugging, and composing. Each chapter is one continuous story: inputs are shown before use, commands are real, and assertions keep the prose honest.

## Chapters

- [Chapter 1: Hello, composix](01-hello-composix.html) — Build, run, probe, and stop your first Cixfile service.

- [Chapter 2: The Cixfile language](02-cixfile-language.html) — Learn binders, assembly, runtime declarations, and the directive vocabulary.

- [Chapter 3: Build, run, debug](03-build-run-debug.html) — Read a Cixfile, build its manifest, run by tag, and debug the same tag.

- [Chapter 4: Building with RUN](04-building-with-run.html) — Build through a persistent workspace and replay only consumed paths.

- [Chapter 5: proj1](05-proj1.html) — Build two services from one Rust workspace and run the API.

- [Chapter 6: Advanced](06-advanced.html) — Inspect socket activation, then compose a real Cixfile-built service.
