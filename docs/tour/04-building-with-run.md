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
PATH ${pkgs.bash}/bin ${pkgs.coreutils}/bin
COPY ${src}/src/ .
RUN <<BUILD
mkdir -p result
tr '[:lower:]' '[:upper:]' < app > result/upper
BUILD

SERVICE run-tour
COPY ${build}/app bin/app
COPY ${build}/result/upper result/upper
EXEC bin/app
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

RUN executes outside Nix evaluation in a networkless sandbox. The offered package closure, incoming directory snapshot, fixed environment, and complete heredoc body form the memo key; the SERVICE copies only the selected results from `${build}`.

```sh
$ cix build .
/nix/store/…-cix-item-run-tour
BUILDER build step 1 COPY /nix/store/…-cix-source/src/ -> . snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 2 RUN memo miss 93ef5c56c71b -> /nix/store/…-cix-build-snapshot
```

```sh
$ tail -n 1 /nix/store/…-cix-item-run-tour/result/upper
ECHO HELLO-FROM-RUN-TOUR
```

The lock records the content-addressed workdir realization. Repeating the unchanged build replays the directory COPY, hits the RUN memo, and returns the identical final item.

```sh
$ cix build .
/nix/store/…-cix-item-run-tour
BUILDER build step 1 COPY /nix/store/…-cix-source/src/ -> . snapshot /nix/store/…-cix-build-snapshot
BUILDER build step 2 RUN memo hit 93ef5c56c71b -> /nix/store/…-cix-build-snapshot
```


---

[← Previous](03-build-run-debug.html) · [Tour index](index.html) · [Next →](05-proj1.html)
