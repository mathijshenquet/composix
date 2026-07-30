# Building and running proj1

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

One Cixfile can compile a workspace and pluck independent store items. `CACHE target` keeps Cargo's incremental state host-local, while each item contains only its declared binary and its bare v4 manifest.

```sh
$ cix build .
proj1-api /nix/store/…-cix-item-proj1-api
proj1-worker /nix/store/…-cix-item-proj1-worker
step 1 COPY rust/Cargo.toml -> Cargo.toml snapshot /nix/store/…-cix-build-snapshot
step 2 COPY rust/Cargo.lock -> Cargo.lock snapshot /nix/store/…-cix-build-snapshot
step 3 COPY rust/common/Cargo.toml -> common/Cargo.toml snapshot /nix/store/…-cix-build-snapshot
step 4 COPY rust/common/src/lib.rs -> common/src/lib.rs snapshot /nix/store/…-cix-build-snapshot
step 5 COPY rust/api/Cargo.toml -> api/Cargo.toml snapshot /nix/store/…-cix-build-snapshot
step 6 COPY rust/api/src/main.rs -> api/src/main.rs snapshot /nix/store/…-cix-build-snapshot
step 7 COPY rust/worker/Cargo.toml -> worker/Cargo.toml snapshot /nix/store/…-cix-build-snapshot
step 8 COPY rust/worker/src/main.rs -> worker/src/main.rs snapshot /nix/store/…-cix-build-snapshot
step 9 RUN memo hit b22c94290699 -> /nix/store/…-cix-build-snapshot
```

```sh
$ cix run /nix/store/…-cix-item-proj1-api --user --detach
cix-run-proj1-api-NONCE.service
warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path
```

```sh
$ curl -fsS http://127.0.0.1:8420
hello from proj1-api
```

```sh
$ systemctl --user stop cix-run-proj1-api-NONCE.service
```


---

[← Previous](13-inspecting.html) · [Tour index](index.html)
