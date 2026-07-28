# Untagging

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

Removing a tag removes its local GC root and its metadata sidecar.

```sh
$ echo 'hello from my app v1' > my-app-v1 && cix tag "$(nix store add my-app-v1)" my-app:v1
```

```sh
$ cix untag my-app:v1
```

```sh
$ cix ls
```

Unpinned means the next `nix-collect-garbage` may reclaim the build; nothing else in cix holds it.


---

[← Previous](02-moving.html) · [Tour index](index.html) · [Next →](04-serving.html)
