# Chapter 5: proj1

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

This small Rust workspace makes the cache and output boundaries concrete. First inspect its complete tree, then read the Cixfile that turns it into two independent service artifacts.

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
PATH ${pkgs.bash}/bin ${pkgs.cargo}/bin ${pkgs.rustc}/bin \
    ${pkgs.gcc}/bin ${pkgs.coreutils}/bin
CACHE target

COPY ${src}/rust/ .
RUN <<BUILD
mkdir -p output
if test -e target/.cix-warm; then
    printf 'warm\n' > output/cache-state
else
    printf 'cold\n' > output/cache-state
fi
cargo build --release --locked --offline --workspace
touch target/.cix-warm
cp target/release/proj1-api output/proj1-api
cp target/release/proj1-worker output/proj1-worker
BUILD

SERVICE proj1-api
COPY ${build}/output/proj1-api bin/proj1-api
EXEC bin/proj1-api
PORT http = 18084

SERVICE proj1-worker
COPY ${build}/output/proj1-worker bin/proj1-worker
EXEC bin/proj1-worker
EGRESS
```

One directory COPY carries the workspace into the builder. The readable RUN heredoc compiles it, while `CACHE target` persists Cargo state outside snapshots and final items. The first build misses the RUN memo and sees an empty cache.

```sh
$ cix build .
proj1-api /nix/store/…-cix-item-proj1-api
proj1-worker /nix/store/…-cix-item-proj1-worker
BUILDER build step 1 COPY /nix/store/…-cix-source/rust/ -> . snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 2 RUN memo miss 7080e246cc33 -> /nix/store/…-cix-build-snapshot
```

```sh
$ printf 'cache-state: ' && cat /nix/store/…-cix-build-snapshot/output/cache-state
cache-state: cold
```

Changing only worker source forces a RUN memo miss, but the declared cache is warm. The API item does not move.

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
$ cix build .
proj1-api /nix/store/…-cix-item-proj1-api
proj1-worker /nix/store/…-cix-item-proj1-worker
BUILDER build step 1 COPY /nix/store/…-cix-source/rust/ -> . snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 2 RUN memo miss 17c8705dcfb7 -> /nix/store/…-cix-build-snapshot
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
BUILDER build step 1 COPY /nix/store/…-cix-source/rust/ -> . snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 2 RUN memo miss 17c8705dcfb7 -> /nix/store/…-cix-build-snapshot
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

[← Previous](04-building-with-run.html) · [Tour index](index.html) · [Next →](06-advanced.html)
