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

ITEM tour-assets
COPY ${src}/greeting.txt /share/greeting

SERVICE tour-app
COPY ${src}/greeting.txt /share/greeting
COPY ${src}/tour-app /bin/tour-app
START ${pkgs.bash}/bin/sh ${src}/tour-app ${pkgs.coreutils}/bin/sleep 300
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

The package universe is pinned by revision and content hash. These ITEM and SERVICE blocks perform only assembly, so they need no BUILDER: builders exist only when FETCH or RUN has work to do.

```sh
$ cix build . --namespace tour -t v1
{"tour-app":"/nix/store/…-cix-item-tour-app","tour-assets":"/nix/store/…-cix-item-tour-assets"}
```

The ITEM is a pure store tree. It deliberately has no runtime manifest, so it can be tagged and copied from but cannot become a systemd unit.

```sh
$ find /nix/store/…-cix-item-tour-assets -type f | sort
/nix/store/…-cix-item-tour-assets/share/greeting
```

```sh
$ cix run tour/tour-assets:v1 --user
Error: /nix/store/…-cix-item-tour-assets has no cix-manifest.json: it is a manifest-less ITEM (D68); items are build products, so use SERVICE/APP to declare a runnable contract
```

## Copy from a tagged item

A tagged cix item is a third FROM input kind. It is a source tree—not a package namespace or inherited root filesystem—so a second Cixfile can copy one declared path from it.

```sh
$ cat prebuilt/Cixfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM tour/tour-assets:v1 AS prior

APP copied-greeting
COPY ${prior}/share/greeting /share/greeting
START /bin/true
```

```sh
$ cix build prebuilt
{"copied-greeting":"/nix/store/…-cix-item-copied-greeting"}
```

```sh
$ cat /nix/store/…-cix-item-copied-greeting/share/greeting
hello from Cixfile
```

The generated lock pins the tag's selected store path and NAR hash. A later tag move does not affect this consumer until `cix build --update-lock prior prebuilt` deliberately refreshes that binder.

```sh
$ cat prebuilt/Cixfile.lock
{
  "inputs": {
    "pkgs": {
      "url": "github:NixOS/nixpkgs/nixos-unstable",
      "rev": "624af665418d3c65d544145b4d34ad696439570e",
      "narHash": "sha256-m0pDuRJG7EDo9ri+4Ksu83VsI+PlxNC9lNBfydejce4="
    }
  },
  "artifacts": {
    "tour/tour-assets:v1": {
      "storePath": "/nix/store/…-cix-item-tour-assets",
      "narHash": "sha256-Zav/GPnMh1fTIW3HoM20hjbFGlBbdrr35lNlXrdaf7U="
    }
  },
  "outputs": {
    "copied-greeting": {
      "sourceHash": "bec8433f28845bd49a9b9d09cdbb9bf4de29a8d35c31b230bbbebfad13e95e7b",
      "storePath": "/nix/store/…-cix-item-copied-greeting"
    }
  }
}
```

Before running anything, inspect the generated manifest. It is the hash-covered runtime contract baked into the item: one version-0 service definition, its executable, and any capabilities or writable directories it declares.

```sh
$ cat /nix/store/…-cix-item-tour-app/cix-manifest.json
{"cixManifest":0,"env":{"PATH":{"default":"bin"}},"mounts":["/bin/tour-app","/share/greeting"],"start":["/nix/store/…-bash-interactive-5.3p15/bin/sh","/nix/store/…-cix-source/tour-app","/nix/store/…-coreutils-9.11/bin/sleep","300"]}
```

## Run

The tag is enough to start a transient service. `--user` is the explicitly degraded rootless development path; production uses the system manager with DynamicUser and the full hardening profile.

```sh
$ cix run tour/tour-app:v1 --detach --user
cix-run-tour-app-NONCE.service
warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path
```

```sh
$ cix ps
MANAGER  UNIT  STATE       RESULT  DESCRIPTION
```

## Debug

`cix debug` resolves the same TAG and compiles the same fresh sandbox, but replaces the declared entrypoint with an operator command. Omitting `-- command` opens an interactive shell.

```sh
$ cix debug tour/tour-app:v1 --user -- /bin/sh -c 'test -n "$CIX_APP" && echo debug-command-ran'
debug-command-ran
warning: cix debug --user is degraded development mode; it does not provide the full system-manager sandbox or DynamicUser identity
=== cix debug: degraded service sandbox; service=tour-app; identity=caller (--user) ===
```

```sh
$ systemctl --user stop cix-run-tour-app-NONCE.service
```

```sh
$ cix ps
MANAGER  UNIT  STATE       RESULT  DESCRIPTION
```


---

[← Previous](02-distribution.html) · [Tour index](index.html) · [Next →](04-building-with-run.html)
