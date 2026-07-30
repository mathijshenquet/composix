# Building with RUN

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

A named `BUILDER` executes `RUN` outside Nix evaluation in a networkless bubblewrap sandbox. Its only store inputs are the closure offered by declared package references; the incoming COPY snapshot and fixed environment complete the memo key. The `SERVICE` then copies only its two results from `${build}`.

```sh
$ cix build .
/nix/store/…-cix-item-run-tour
BUILDER build step 1 COPY /nix/store/…-cix-source/app -> app snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 2 RUN memo miss 0b957b5a13a6 (… ms) -> /nix/store/…-cix-build-snapshot
```

```sh
$ tail -n 1 /nix/store/…-cix-item-run-tour/result/upper
ECHO HELLO-FROM-RUN-TOUR
```

The lock now records the content-addressed workdir realization. Repeating the same build replays the COPY snapshot and hits RUN's memo; the final item path stays identical.

```sh
$ cix build .
/nix/store/…-cix-item-run-tour
BUILDER build step 1 COPY /nix/store/…-cix-source/app -> app snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 2 RUN memo hit 0b957b5a13a6 -> /nix/store/…-cix-build-snapshot
```


---

[← Previous](11-debugging-service.html) · [Tour index](index.html) · [Next →](13-inspecting.html)
