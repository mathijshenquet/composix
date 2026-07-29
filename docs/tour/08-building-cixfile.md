# Building from a Cixfile

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

A Cixfile can build a runnable item without declaring a package. The checked-in lock still pins nixpkgs because `SCRIPT` uses its runtime shell; it makes generation deterministic, and a fresh store may fetch that pinned source once.

```sh
$ cix build . -t tour-app:v1
/nix/store/…-cixfile-item
```

The generated spec is the build's runtime contract: it records the service name and executable independently of the Cixfile source.

```sh
$ cat /nix/store/…-cixfile-item/cix-spec.json
{"cixSpec":2,"services":{"tour-app":{"exec":["bin/tour-app"]}}}
```

```sh
$ cix ls
tour-app:v1
```


---

[← Previous](07-running-service.html) · [Tour index](index.html) · [Next →](09-running-listener.html)
