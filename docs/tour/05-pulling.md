# Pulling on another machine

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

A second machine is just a second state dir.

```sh
publisher $ mkdir -p fixture-v1 && printf '%s\n' 'hello from my app v1' > fixture-v1/README
```

```sh
publisher $ nix store add-path fixture-v1
/nix/store/…-fixture-v1
```

```sh
publisher $ cix tag /nix/store/…-fixture-v1 my-app:v1
```

```sh
publisher $ cix serve --with-store --listen 127.0.0.1:8420 &
```

```sh
consumer $ cix pull 127.0.0.1:8420/my-app:v1 --as my-app
updated 1 tag(s)
```

```sh
consumer $ cix ls -l
my-app:latest	systems=x86_64-linux	path=/nix/store/…-fixture-v1	upstream=127.0.0.1:8420	age=0s
```

The qualified ref is self-describing; `--as` adopts it under a bare local name. A mirror keeps its qualified remote identity, while adoption makes the name local.


---

[← Previous](04-serving.html) · [Tour index](index.html) · [Next →](06-pull-follows.html)
