# Building and running proj1

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

The Cixfile names one builder and two independent service artifacts. Its declared `CACHE target` persists Cargo state without putting that state in snapshots or items.

```sh
$ cat Cixfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

BUILDER build
PATH ${pkgs.bash}/bin ${pkgs.cargo}/bin ${pkgs.rustc}/bin ${pkgs.gcc}/bin ${pkgs.coreutils}/bin
CACHE target

COPY ${src}/rust/Cargo.toml Cargo.toml
COPY ${src}/rust/Cargo.lock Cargo.lock
COPY ${src}/rust/common/Cargo.toml common/Cargo.toml
COPY ${src}/rust/common/src/lib.rs common/src/lib.rs
COPY ${src}/rust/api/Cargo.toml api/Cargo.toml
COPY ${src}/rust/api/src/main.rs api/src/main.rs
COPY ${src}/rust/worker/Cargo.toml worker/Cargo.toml
COPY ${src}/rust/worker/src/main.rs worker/src/main.rs
RUN mkdir -p output && if test -e target/.cix-warm; then printf 'warm\n' > output/cache-state; else printf 'cold\n' > output/cache-state; fi && cargo build --release --locked --offline --workspace && touch target/.cix-warm && cp target/release/proj1-api output/proj1-api && cp target/release/proj1-worker output/proj1-worker

SERVICE proj1-api
COPY ${build}/output/proj1-api bin/proj1-api
EXEC bin/proj1-api
PORT http = 18084

SERVICE proj1-worker
COPY ${build}/output/proj1-worker bin/proj1-worker
EXEC bin/proj1-worker
EGRESS
```

The first build misses the RUN memo and sees an empty cache.

```sh
$ cix build .
proj1-api /nix/store/…-cix-item-proj1-api
proj1-worker /nix/store/…-cix-item-proj1-worker
BUILDER build step 1 COPY /nix/store/…-cix-source/rust/Cargo.toml -> Cargo.toml snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 2 COPY /nix/store/…-cix-source/rust/Cargo.lock -> Cargo.lock snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 3 COPY /nix/store/…-cix-source/rust/common/Cargo.toml -> common/Cargo.toml snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 4 COPY /nix/store/…-cix-source/rust/common/src/lib.rs -> common/src/lib.rs snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 5 COPY /nix/store/…-cix-source/rust/api/Cargo.toml -> api/Cargo.toml snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 6 COPY /nix/store/…-cix-source/rust/api/src/main.rs -> api/src/main.rs snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 7 COPY /nix/store/…-cix-source/rust/worker/Cargo.toml -> worker/Cargo.toml snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 8 COPY /nix/store/…-cix-source/rust/worker/src/main.rs -> worker/src/main.rs snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 9 RUN memo miss 033cffdee330 -> /nix/store/…-cix-build-snapshot
```

```sh
$ printf 'cache-state: ' && cat /nix/store/…-cix-build-snapshot/output/cache-state
cache-state: cold
```

Changing only worker source forces a RUN memo miss, but the declared cache is warm. The API item does not move.

```sh
$ sed -i 's/proj1-worker/proj1-worker-edited/' rust/worker/src/main.rs
```

```sh
$ cix build .
proj1-api /nix/store/…-cix-item-proj1-api
proj1-worker /nix/store/…-cix-item-proj1-worker
BUILDER build step 1 COPY /nix/store/…-cix-source/rust/Cargo.toml -> Cargo.toml snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 2 COPY /nix/store/…-cix-source/rust/Cargo.lock -> Cargo.lock snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 3 COPY /nix/store/…-cix-source/rust/common/Cargo.toml -> common/Cargo.toml snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 4 COPY /nix/store/…-cix-source/rust/common/src/lib.rs -> common/src/lib.rs snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 5 COPY /nix/store/…-cix-source/rust/api/Cargo.toml -> api/Cargo.toml snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 6 COPY /nix/store/…-cix-source/rust/api/src/main.rs -> api/src/main.rs snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 7 COPY /nix/store/…-cix-source/rust/worker/Cargo.toml -> worker/Cargo.toml snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 8 COPY /nix/store/…-cix-source/rust/worker/src/main.rs -> worker/src/main.rs snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 9 RUN memo miss 5d2fa2c018f6 -> /nix/store/…-cix-build-snapshot
```

```sh
$ printf 'cache-state: ' && cat /nix/store/…-cix-build-snapshot/output/cache-state
cache-state: warm
```

```sh
$ test /nix/store/…-cix-item-proj1-api = /nix/store/…-cix-item-proj1-api && echo 'api item unchanged: yes'
api item unchanged: yes
```

A clean `--no-cache` rebuild starts cold again and produces byte-identical items.

```sh
$ cix build --no-cache .
proj1-api /nix/store/…-cix-item-proj1-api
proj1-worker /nix/store/…-cix-item-proj1-worker
BUILDER build step 1 COPY /nix/store/…-cix-source/rust/Cargo.toml -> Cargo.toml snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 2 COPY /nix/store/…-cix-source/rust/Cargo.lock -> Cargo.lock snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 3 COPY /nix/store/…-cix-source/rust/common/Cargo.toml -> common/Cargo.toml snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 4 COPY /nix/store/…-cix-source/rust/common/src/lib.rs -> common/src/lib.rs snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 5 COPY /nix/store/…-cix-source/rust/api/Cargo.toml -> api/Cargo.toml snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 6 COPY /nix/store/…-cix-source/rust/api/src/main.rs -> api/src/main.rs snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 7 COPY /nix/store/…-cix-source/rust/worker/Cargo.toml -> worker/Cargo.toml snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 8 COPY /nix/store/…-cix-source/rust/worker/src/main.rs -> worker/src/main.rs snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 9 RUN memo miss 5d2fa2c018f6 -> /nix/store/…-cix-build-snapshot
```

```sh
$ printf 'cache-state: ' && cat /nix/store/…-cix-build-snapshot/output/cache-state
cache-state: cold
```

```sh
$ test /nix/store/…-cix-item-proj1-api = /nix/store/…-cix-item-proj1-api && test /nix/store/…-cix-item-proj1-worker = /nix/store/…-cix-item-proj1-worker && echo 'item paths byte-identical: yes'
item paths byte-identical: yes
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
