# Chapter 3: Build, run, debug

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

A Cixfile turns source material plus pinned package inputs into a store item with a runtime manifest. We start by looking at every input this tiny build will use.

## Build

```sh
$ ls -1 Cixfile Cixfile.lock greeting.txt tour-app
Cixfile
Cixfile.lock
greeting.txt
tour-app
```

```sh
$ cat Cixfile greeting.txt tour-app
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

SERVICE tour-app
COPY ${src}/greeting.txt share/greeting
COPY ${src}/tour-app bin/tour-app
EXEC ${pkgs.bash}/bin/sh ${src}/tour-app ${pkgs.coreutils}/bin/sleep 300
hello from Cixfile
exec "$@"
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

The package universe is pinned by revision and content hash. This SERVICE performs only assembly, so it needs no BUILDER: builders exist only when FETCH or RUN has work to do.

```sh
$ cix build . -t tour-app:v1
/nix/store/…-cix-item-tour-app
```

Before running anything, inspect the generated manifest. It is the hash-covered runtime contract baked into the item: one v4 service definition, its executable, and any capabilities or writable directories it declares.

```sh
$ cat /nix/store/…-cix-item-tour-app/cix-manifest.json
{"cixManifest":4,"exec":["/nix/store/…-bash-interactive-5.3p15/bin/sh","/nix/store/…-cix-source/tour-app","/nix/store/…-coreutils-9.11/bin/sleep","300"],"mounts":["/bin/tour-app","/share/greeting"]}
```

## Run

The tag is enough to start a transient service. `--user` is the explicitly degraded rootless development path; production uses the system manager with DynamicUser and the full hardening profile.

```sh
$ cix run tour-app:v1 --detach --user
cix-run-tour-app-NONCE.service
warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path
```

```sh
$ cix ps
MANAGER  UNIT                                        STATE       DESCRIPTION
user     cix-run-tour-app-NONCE.service  active/running  /nix/store/…-bash-interactive-5.3p15/bin/sh /nix/store/…-cix-source/tour-app /nix/store/…-coreutils-9.11/bin/sleep 300
```

## Debug

`cix debug` resolves the same TAG and compiles the same fresh sandbox, but replaces the declared entrypoint with an operator command. Omitting `-- command` opens an interactive shell.

```sh
$ cix debug tour-app:v1 --user -- /bin/sh -c 'test -n "$CIX_APP" && echo debug-command-ran'
debug-command-ran
warning: cix debug --user is degraded development mode; it does not provide the full system-manager sandbox or DynamicUser identity
=== cix debug: degraded service sandbox; service=tour-app; identity=caller (--user) ===
```

```sh
$ systemctl --user stop cix-run-tour-app-NONCE.service
```

```sh
$ cix ps
MANAGER  UNIT  STATE       DESCRIPTION
```


---

[← Previous](02-distribution.html) · [Tour index](index.html) · [Next →](04-building-with-run.html)
