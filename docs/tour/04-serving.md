# Serving your store

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

Publication is not a ceremony — serving exposes your bare tags at whatever URL reaches the box.

```sh
publisher $ echo 'hello from my app v1' > my-app-v1 && cix tag "$(nix store add my-app-v1)" my-app:v1
```

```sh
publisher $ cix serve --with-store --listen 127.0.0.1:8420 &
```

```sh
publisher $ curl -s -H 'Accept: application/vnd.cix+json;version=1' http://127.0.0.1:8420/my-app:v1
{"outputs":{"x86_64-linux":{"storePath":"/nix/store/…-my-app-v1","narHash":"sha256-UjgGe265G0pyovh3lkIj92mKGv7d64Q9nd9w14qBQ4I="}},"substituters":["http://127.0.0.1:8420/store"],"createdAt":"1700000000"}
```

The same URL in a browser is an informative HTML page; here is only a short teaser, not the page dump.

```sh
publisher $ curl -s http://127.0.0.1:8420/my-app:v1 | head -c 120
<!doctype html><html lang="en"><head><meta charset="utf-8"><title>127.0.0.1:8420/my-app:v1</title><style>body{font:16px
```


---

[← Previous](03-untagging.html) · [Tour index](index.html) · [Next →](05-pulling.html)
