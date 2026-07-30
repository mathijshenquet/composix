# Building from a Cixfile

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

A Cixfile may bind a package universe with `FROM <flakeref> AS pkgs`. The checked-in lock pins that remote universe (rev + content hash), which makes generation deterministic; a fresh store may fetch the pinned source once.

```sh
$ cix build . -t tour-app:v1
/nix/store/…-cix-item-tour-app
```

The generated v4 manifest is the build's runtime contract: this `SERVICE` produces one bare service definition.

```sh
$ cat /nix/store/…-cix-item-tour-app/cix-manifest.json
{"cixManifest":4,"exec":["bin/tour-app"]}
```

```sh
$ cix ls
tour-app:v1
```


---

[← Previous](07-running-service.html) · [Tour index](index.html) · [Next →](09-running-listener.html)
