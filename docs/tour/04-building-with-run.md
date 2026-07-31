# Chapter 4: Building with RUN

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

A BUILDER is the workshop side of a Cixfile: it exists because this example has RUN work to perform. First inspect the complete local working directory and the files the build consumes.

```sh
$ ls -R .
.:
Cixfile
Cixfile.lock
bin
src

./bin:
cix

./src:
app
```

```sh
$ cat src/app
#!/bin/sh
echo hello-from-run-tour
```

```sh
$ cat Cixfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
COPY ${src}/src/ .
RUN <<BUILD
if test -e .cix-warm; then
    printf 'workspace-state: warm\n'
else
    printf 'workspace-state: cold\n'
fi
mkdir -p result
tr '[:lower:]' '[:upper:]' < app > result/upper
touch .cix-warm
BUILD

SERVICE run-tour
COPY ${build}/app /bin/app
COPY ${build}/result/upper /result/upper
EXEC app
```

```sh
$ cat Cixfile.lock
{
  "inputs": {
    "pkgs": {
      "url": "github:NixOS/nixpkgs/nixos-unstable",
      "rev": "624af665418d3c65d544145b4d34ad696439570e",
      "narHash": "sha256-m0pDuRJG7EDo9ri+4Ksu83VsI+PlxNC9lNBfydejce4="
    }
  }
}
```

IMPORT makes bare tools available through the read-only `/bin` union. The chain key contains the command, imports, predecessor, environment, and declared COPY bytes—but never workspace bytes. The SERVICE consumes only two narrow paths from `${build}`.

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-run cix build .
{"run-tour":"/nix/store/…-cix-item-run-tour"}
BUILDER build workspace <persistent>
BUILDER build step 1 COPY /nix/store/…-cix-source/src/ -> .
workspace-state: cold
BUILDER build step 2 RUN executed
BUILDER build memo miss 2fa3a37fc0c1 -> /nix/store/…-cix-build-view
```

```sh
$ tail -n 1 /nix/store/…-cix-item-run-tour/result/upper
ECHO HELLO-FROM-RUN-TOUR
```

The lock records just those consumed paths. Repeating the unchanged build materializes them from the store without running the builder.

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-run cix build .
{"run-tour":"/nix/store/…-cix-item-run-tour"}
BUILDER build memo hit 2fa3a37fc0c1 -> /nix/store/…-cix-build-view
```

Changing a declared input changes the chain key. The builder runs again in its persistent workspace, so its private marker is warm while the selected outputs still depend only on declared inputs.

```sh
$ sed -i 's/hello-from-run-tour/hello-from-run-tour-edited/' src/app
```

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-run cix build .
{"run-tour":"/nix/store/…-cix-item-run-tour"}
BUILDER build workspace <persistent>
BUILDER build step 1 COPY /nix/store/…-cix-source/src/ -> .
workspace-state: warm
BUILDER build step 2 RUN executed
BUILDER build memo miss 2a11274b7ae3 -> /nix/store/…-cix-build-view
```

`--cold` samples the same chain with an empty workspace and compares each consumed path. The marker says cold, while the artifact is byte-identical.

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-run cix build --cold .
{"run-tour":"/nix/store/…-cix-item-run-tour"}
BUILDER build step 1 COPY /nix/store/…-cix-source/src/ -> .
workspace-state: cold
BUILDER build step 2 RUN executed
BUILDER build memo miss 2a11274b7ae3 -> /nix/store/…-cix-build-view
```

A workspace is only an acceleration structure. Removing it is always safe: the unchanged chain still replays the recorded paths and returns the same item.

```sh
$ rm -rf ../.workspaces-run
```

```sh
$ CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-run cix build .
{"run-tour":"/nix/store/…-cix-item-run-tour"}
BUILDER build memo hit 2a11274b7ae3 -> /nix/store/…-cix-build-view
```


---

[← Previous](03-build-run-debug.html) · [Tour index](index.html) · [Next →](05-proj1.html)
