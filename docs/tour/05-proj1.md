# Chapter 5: proj1

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

This small Rust workspace makes persistent workspaces and narrow output records concrete. First inspect its complete tree, then read the Cixfile that turns it into two independent service artifacts.

```sh
$ ls -R .
.:
Cixfile
Cixfile.lock
bin
rust

./bin:
cix

./rust:
Cargo.lock
Cargo.toml
api
common
worker

./rust/api:
Cargo.toml
src

./rust/api/src:
main.rs

./rust/common:
Cargo.toml
src

./rust/common/src:
lib.rs

./rust/worker:
Cargo.toml
src

./rust/worker/src:
main.rs
```

```sh
$ cat Cixfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

BUILDER build
IMPORT ${pkgs.bash} ${pkgs.cargo} ${pkgs.rustc} \
    ${pkgs.gcc} ${pkgs.coreutils}

COPY ${src}/rust/ .
RUN <<BUILD
if test -e target/.cix-warm; then
    printf 'workspace-state: warm\n'
else
    printf 'workspace-state: cold\n'
fi
cargo build --release --locked --offline --workspace
touch target/.cix-warm
BUILD

SERVICE proj1-api
COPY ${build}/target/release/proj1-api /bin/proj1-api
EXEC proj1-api
PORT http = 18084

SERVICE proj1-worker
COPY ${build}/target/release/proj1-worker /bin/proj1-worker
EXEC proj1-worker
GRANT egress
```

One directory COPY stages the declared Rust sources. Cargo's `target/` tree and the marker written by RUN remain in the persistent workspace automatically, while the two SERVICE blocks consume only their own release binaries. The first build is cold.

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-proj1 cix build . --namespace proj1 -t v1
{"proj1-api":"/nix/store/…-cix-item-proj1-api","proj1-worker":"/nix/store/…-cix-item-proj1-worker"}
BUILDER build workspace <persistent>
BUILDER build step 1 COPY /nix/store/…-cix-source/rust/ -> .
workspace-state: cold
BUILDER build step 2 RUN executed
BUILDER build memo miss 135872ac4ba5 -> /nix/store/…-cix-build-view
```

Changing only worker source changes the chain key and runs the builder in its warm workspace. Cargo rebuilds what changed. Because the lock records each consumed binary separately, the API item does not move.

```sh
$ cat rust/worker/src/main.rs
fn main() {
    println!("{}", common::greeting("proj1-worker"));
}
```

```sh
$ sed -i 's/proj1-worker/proj1-worker-edited/' rust/worker/src/main.rs
```

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-proj1 cix build .
{"proj1-api":"/nix/store/…-cix-item-proj1-api","proj1-worker":"/nix/store/…-cix-item-proj1-worker"}
BUILDER build workspace <persistent>
BUILDER build step 1 COPY /nix/store/…-cix-source/rust/ -> .
workspace-state: warm
BUILDER build step 2 RUN executed
BUILDER build memo miss 0a8dbf23a19f -> /nix/store/…-cix-build-view
```

```sh
$ test /nix/store/…-cix-item-proj1-api = /nix/store/…-cix-item-proj1-api && echo 'api item unchanged: yes'
api item unchanged: yes
```

A sampled `--cold` rebuild uses an empty workspace. The marker says cold, and per-path comparison proves both selected binaries—and therefore both item paths—are byte-identical.

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-proj1 cix build --cold .
{"proj1-api":"/nix/store/…-cix-item-proj1-api","proj1-worker":"/nix/store/…-cix-item-proj1-worker"}
BUILDER build step 1 COPY /nix/store/…-cix-source/rust/ -> .
workspace-state: cold
BUILDER build step 2 RUN executed
BUILDER build memo miss 0a8dbf23a19f -> /nix/store/…-cix-build-view
```

```sh
$ test /nix/store/…-cix-item-proj1-api = /nix/store/…-cix-item-proj1-api && test /nix/store/…-cix-item-proj1-worker = /nix/store/…-cix-item-proj1-worker && echo 'item paths byte-identical: yes'
item paths byte-identical: yes
```

The warm workspace remains disposable. Delete it and the unchanged chain replays the two recorded binaries without changing either item.

```sh
$ rm -rf ../.workspaces-proj1
```

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-proj1 cix build .
{"proj1-api":"/nix/store/…-cix-item-proj1-api","proj1-worker":"/nix/store/…-cix-item-proj1-worker"}
BUILDER build memo hit 0a8dbf23a19f -> /nix/store/…-cix-build-view
```

```sh
$ cix run proj1/proj1-api:v1 --user --detach
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

[← Previous](04-building-with-run.html) · [Tour index](index.html) · [Next →](06-advanced.html)
