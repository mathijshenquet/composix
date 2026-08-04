# Chapter 3: Building: BUILDERs, FETCH, and the lock

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

You will build pinned network inputs in persistent workspaces, audit them cold, and repair a real downloaded FHS binary before compiling a two-service Rust workspace. Afterwards, you will understand what the lock records, why RUN is offline, how warm replay stays trustworthy, and how one builder can feed narrow independent members.

## FETCH, EXPECT, and deliberate lock movement

This compact fixture uses both trust modes. `expected` carries the author's whole-tree SRI hash directly in the Cixfile; `resolved` asks cix to fetch twice and record only the downstream-observable path when you explicitly update that lock entry.

```sh
$ cat fetch-demo/Cixfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

FETCH expected ${pkgs.coreutils}/bin/printf author-pinned > expected EXPECT sha256-FMDQ1JAsOcmebFh//goocO3F9g7aCK37MJxrDuzvqw8=
FETCH resolved ${pkgs.coreutils}/bin/printf lock-pinned > resolved

BUILDER assemble
IMPORT ${pkgs.bash} ${pkgs.coreutils}
COPY ${expected}/expected expected
COPY ${resolved}/resolved resolved
RUN cat expected resolved > result

ITEM fetched-result
COPY ${assemble}/result /result
```

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build --update-lock resolved fetch-demo
{"fetched-result":"/nix/store/…-cix-item-fetched-result"}
FETCH expected memo miss 999a72d2b130 -> /nix/store/…-cix-build-view
FETCH resolved update probe: two outputs were identical
FETCH resolved memo miss 7a6aa9b6b1cf -> /nix/store/…-cix-build-view
BUILDER assemble workspace <persistent>
BUILDER assemble step 1 COPY /nix/store/…-cix-build-view/expected -> expected
BUILDER assemble step 2 COPY /nix/store/…-cix-build-view/resolved -> resolved
BUILDER assemble step 3 RUN executed
BUILDER assemble memo miss 996aef633ffd -> /nix/store/…-cix-build-view
```

```sh
$ cat /nix/store/…-cix-item-fetched-result/result
author-pinnedlock-pinned
```

The lock keeps the immutable nixpkgs revision, FETCH pins, constructive step memos, consumed output objects, and a development-environment snapshot. The snapshot comes from the imported package world, so native toolchain variables arrive together; you do not hand-wire store paths such as `PKG_CONFIG_PATH`.

```sh
$ jq '{fetches, devEnvCount:(.devEnvs | length)}' fetch-demo/Cixfile.lock
{
  "fetches": {
    "expected": {
      "narHash": "sha256-FMDQ1JAsOcmebFh//goocO3F9g7aCK37MJxrDuzvqw8="
    },
    "resolved": {
      "paths": {
        "resolved": "sha256-SFdDQrQHvg7nYptmG8okQasC+OpS9QI1Ke44XO7Qox4="
      }
    }
  },
  "devEnvCount": 1
}
```

FETCH alone has network authority. The following RUN consumes only staged files in a networkless sandbox, and its command plus imports, environment, and observed read set form the reusable step identity.

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build --stats fetch-demo
{"items":{"fetched-result":"/nix/store/…-cix-item-fetched-result"},"stats":{"nixSubprocesses":0,"steps":[{"kind":"FETCH","name":"expected","status":"memo-hit"},{"kind":"FETCH","name":"resolved","status":"memo-hit"},{"kind":"COPY","name":"assemble:1","status":"memo-hit"},{"kind":"COPY","name":"assemble:2","status":"memo-hit"},{"kind":"RUN","name":"assemble:3","status":"memo-hit"}]}}
BUILDER assemble memo hit completed output (zero Nix subprocesses)
```

That hit is not a timestamp promise. Cix rehashes exactly the files, directory listings, metadata probes, and absent paths the command read; unrelated workspace bytes cannot invalidate the step, while a changed observed input does. A persistent workspace is therefore an acceleration structure, not hidden build input.

`--update-lock` and `--cold` are the audit pair: the first is an explicit trust-moving network operation for a selected non-EXPECT FETCH, while the second never contacts the network and replays the pinned bytes in an empty workspace before comparing reads and consumed outputs.

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build --cold fetch-demo
{"fetched-result":"/nix/store/…-cix-item-fetched-result"}
FETCH expected replayed pinned snapshot 999a72d2b130 -> /nix/store/…-cix-build-view
FETCH resolved replayed pinned snapshot 7a6aa9b6b1cf -> /nix/store/…-cix-build-view
BUILDER assemble step 1 COPY /nix/store/…-cix-build-view/expected -> expected
BUILDER assemble step 2 COPY /nix/store/…-cix-build-view/resolved -> resolved
BUILDER assemble step 3 RUN executed
BUILDER assemble memo miss 996aef633ffd -> /nix/store/…-cix-build-view
```

## The FHS diagnostic, then the one-line fix

The next FETCH downloads an ELF whose interpreter is the conventional GNU `/lib64/ld-linux-x86-64.so.2`. The builder imports a shell and core utilities but no libc, so executing the untouched download must fail—and the shown diagnostic is produced by the real trace, not copied into this guide.

```sh
$ cat fhs-demo/Cixfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

FETCH native ${pkgs.curl}/bin/curl -fsS http://127.0.0.1:8420/fhs-probe -o fhs-probe EXPECT sha256-1gzOMISEGO2q2/dK8HpOVFArd8ROwzTqw6TmaVloGa8=

BUILDER native-build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
COPY ${native}/fhs-probe .
RUN chmod +x fhs-probe
RUN ./fhs-probe > result

ITEM native-result
COPY ${native-build}/result /result
```

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build fhs-demo
FETCH native memo miss 836a265a6bc7 -> /nix/store/…-cix-build-view
BUILDER native-build workspace <persistent>
BUILDER native-build step 1 COPY /nix/store/…-cix-build-view/fhs-probe -> .
BUILDER native-build step 2 RUN executed
cix-build: line 1: ./fhs-probe: cannot execute: required file not found
Error: line 9: RUN failed
  | "RUN ./fhs-probe > result"

Caused by:
    bubblewrap sandbox or command exited exit status: 127; sandboxing was not weakened (enable unprivileged user namespaces if bwrap reported a namespace permission error)
    hint: fhs-probe requires the FHS loader /lib64/ld-linux-x86-64.so.2 and libc.so.6; IMPORT ${pkgs.glibc}
    command stderr:
    cix-build: line 1: ./fhs-probe: cannot execute: required file not found
```

Add glibc to the ordered IMPORT union. Its loader satisfies the fixed FHS alias, so the same downloaded binary runs without mutation or a patchelf step.

```sh
$ sed -i 's/${pkgs.coreutils}/${pkgs.coreutils} ${pkgs.glibc}/' fhs-demo/Cixfile
```

```sh
$ grep '^IMPORT' fhs-demo/Cixfile
IMPORT ${pkgs.bash} ${pkgs.coreutils} ${pkgs.glibc}
```

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build fhs-demo
{"native-result":"/nix/store/…-cix-item-native-result"}
FETCH native memo hit 836a265a6bc7 -> /nix/store/…-cix-build-view
BUILDER native-build workspace <persistent>
BUILDER native-build step 1 COPY /nix/store/…-cix-build-view/fhs-probe -> .
BUILDER native-build step 2 RUN executed
BUILDER native-build step 3 RUN executed
BUILDER native-build memo miss f5d5a2149974 -> /nix/store/…-cix-build-view
```

```sh
$ cat /nix/store/…-cix-item-native-result/result
fhs-tour-ok
```

## Capstone: one Rust workspace, two services

The capstone is a real Cargo workspace. One BUILDER imports its complete pinned toolchain, stages the declared workspace, and runs Cargo offline; two SERVICE blocks consume one release binary each.

```sh
$ cat proj1/Cixfile
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
  START proj1-api
  PORT http = 18084

SERVICE proj1-worker
  COPY ${build}/target/release/proj1-worker /bin/proj1-worker
  START proj1-worker
  CLAIM egress
```

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build proj1 --namespace proj1 -t v1
{"proj1-api":"/nix/store/…-cix-item-proj1-api","proj1-worker":"/nix/store/…-cix-item-proj1-worker"}
BUILDER build workspace <persistent>
BUILDER build step 1 COPY /nix/store/…-cix-source/rust/ -> .
workspace-state: cold
BUILDER build step 2 RUN executed
BUILDER build memo miss 5a5e839719dd -> /nix/store/…-cix-build-view
```

Now change only the worker and select that member with `directory#member`. The warm builder recompiles the changed workspace, but the API's narrow consumed path still names the same immutable item.

```sh
$ sed -i 's/proj1-worker/proj1-worker-edited/' proj1/rust/worker/src/main.rs
```

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build proj1#proj1-worker
/nix/store/…-cix-item-proj1-worker
BUILDER build workspace <persistent>
BUILDER build step 1 COPY /nix/store/…-cix-source/rust/ -> .
workspace-state: warm
BUILDER build step 2 RUN executed
BUILDER build memo miss 75f8514b46cf -> /nix/store/…-cix-build-view
```

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build proj1#proj1-api
/nix/store/…-cix-item-proj1-api
BUILDER build workspace <persistent>
BUILDER build step 1 COPY /nix/store/…-cix-source/rust/ -> .
workspace-state: warm
BUILDER build step 2 RUN executed
BUILDER build memo miss 75f8514b46cf -> /nix/store/…-cix-build-view
```

That is the central build model at project scale: shared warm work stays private to the builder, FETCH trust stays pinned, and each final item depends only on the path it actually copies.


---

[← Previous](02-cixfile-language.html) · [Tour index](index.html) · [Next →](04-building-with-run.html)
