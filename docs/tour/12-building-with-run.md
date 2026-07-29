# Building with RUN

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

`RUN` executes outside Nix evaluation in a networkless bubblewrap sandbox. Its only store inputs are the closure offered by the declared package references; the incoming COPY snapshot and fixed environment complete the memo key.

```sh
$ cix build .
/nix/store/…-cixfile-item
step 1 COPY app -> app snapshot /nix/store/…-cix-build-snapshot
step 2 RUN memo miss c307c177f29e (… ms) -> /nix/store/…-cix-build-snapshot
```

```sh
$ tail -n 1 /nix/store/…-cixfile-item/result/upper
ECHO HELLO-FROM-RUN-TOUR
```

The lock now records the content-addressed workdir realization. Repeating the same build replays the COPY snapshot and hits RUN's memo; the final item path stays identical.

```sh
$ cix build .
/nix/store/…-cixfile-item
step 1 COPY app -> app snapshot /nix/store/…-cix-build-snapshot
step 2 RUN memo hit c307c177f29e -> /nix/store/…-cix-build-snapshot
```


---

[← Previous](11-debugging-service.html) · [Tour index](index.html)
