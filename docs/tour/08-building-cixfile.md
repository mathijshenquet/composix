# Building from a Cixfile

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

Every Cixfile begins by binding its package universe: `FROM <flakeref> AS pkgs`. The checked-in lock pins that universe (rev + content hash), which makes generation deterministic; a fresh store may fetch the pinned source once.

```sh
$ cix build . -t tour-app:v1
/nix/store/…-cixfile-item
```

The generated spec is the build's runtime contract: it records the service name and executable independently of the Cixfile source.

```sh
$ cat /nix/store/…-cixfile-item/cix-manifest.json
{"cixManifest":2,"services":{"tour-app":{"exec":["bin/tour-app"]}}}
```

```sh
$ cix ls
tour-app:v1
```


---

[← Previous](07-running-service.html) · [Tour index](index.html) · [Next →](09-running-listener.html)
