# Pulling on another machine

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

A second machine is just a second state dir.

```sh
publisher $ echo 'hello from my app v1' > my-app-v1 && cix tag "$(nix store add my-app-v1)" my-app:v1
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
REF            SYSTEMS       PATH                                                   UPSTREAM         AGE
my-app:latest  x86_64-linux  /nix/store/…-my-app-v1  127.0.0.1:8420  0s
```

The qualified ref is self-describing; `--as` adopts it under a bare local name. A mirror keeps its qualified remote identity, while adoption makes the name local.


---

[← Previous](04-serving.html) · [Tour index](index.html) · [Next →](06-pull-follows.html)
