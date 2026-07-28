# Tags move; pull follows

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

A consumer can track a remote tag without making the publisher's name local.

```sh
publisher $ echo 'hello from my app v1' > my-app-v1 && cix tag "$(nix store add my-app-v1)" my-app:v1
```

```sh
publisher $ cix serve --with-store --listen 127.0.0.1:8420 &
```

```sh
consumer $ cix pull 127.0.0.1:8420/my-app:v1
updated 1 tag(s)
```

```sh
publisher $ echo 'hello from my app v2' > my-app-v2 && cix tag "$(nix store add my-app-v2)" my-app:v1
```

```sh
consumer $ cix pull
updated 1 tag(s)
```

```sh
consumer $ cix ls -l
127.0.0.1:8420/my-app:v1	systems=x86_64-linux	path=/nix/store/…-my-app-v2	upstream=127.0.0.1:8420	age=0s
```

Tags are mutable names over immutable paths, refreshed like git remotes. GC follows the pins: after the refresh, this consumer tag roots the new path, not the old one.


---

[← Previous](05-pulling.html) · [Tour index](index.html) · [Next →](07-running-service.html)
