# Chapter 2: Distribution

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

A served index and a standard Nix binary cache are enough to move the same immutable artifact between machines. Separate state directories stand in for the publisher and consumer here.

## Serve

```sh
publisher $ mkdir my-app-v1 && printf '%s\n' 'hello from my app v1' > my-app-v1/message && printf '%s\n' '{"cixManifest":0,"start":["message"]}' > my-app-v1/cix-manifest.json
```

```sh
publisher $ ls -1 my-app-v1
cix-manifest.json
message
```

```sh
publisher $ cat my-app-v1/message my-app-v1/cix-manifest.json
hello from my app v1
{"cixManifest":0,"start":["message"]}
```

```sh
publisher $ cix tag "$(nix store add my-app-v1)" my-app:v1
```

```sh
publisher $ cix serve --with-store --listen 127.0.0.1:8420 &
```

```sh
publisher $ curl -s -H 'Accept: application/vnd.cix+json;version=1' http://127.0.0.1:8420/my-app:v1
{"outputs":{"x86_64-linux":{"storePath":"/nix/store/…-my-app-v1","narHash":"sha256-iUMlkbB006RUsAvCgp43+tHkb8uxbWjnGu7KbTSMo7w="}},"substituters":["http://127.0.0.1:8420/store"],"createdAt":"1700000000"}
```

The same URL serves an informative HTML representation to a browser; content negotiation keeps one public name instead of a separate API URL.

```sh
publisher $ curl -s http://127.0.0.1:8420/my-app:v1 | head -c 120
<!doctype html><html lang="en"><head><meta charset="utf-8"><title>127.0.0.1:8420/my-app:v1</title><style>body{font:16px
```

## Pull

The qualified ref names both its origin and tag. `--as` adopts it under a bare local name while retaining the upstream needed for later refreshes.

```sh
consumer $ cix pull 127.0.0.1:8420/my-app:v1 --as my-app:v1
updated 1 tag(s)
```

```sh
consumer $ cix ls -l
REF        SYSTEMS       PATH                                                   UPSTREAM         AGE
my-app:v1  x86_64-linux  /nix/store/…-my-app-v1  127.0.0.1:8420  0s
```

## Follow a moving tag

The publisher can move `my-app:v1` to a new immutable path. A bare `cix pull` refreshes every local tag that remembers an upstream.

```sh
publisher $ mkdir my-app-v2 && printf '%s\n' 'hello from my app v2' > my-app-v2/message && printf '%s\n' '{"cixManifest":0,"start":["message"]}' > my-app-v2/cix-manifest.json
```

```sh
publisher $ ls -1 my-app-v2
cix-manifest.json
message
```

```sh
publisher $ cat my-app-v2/message my-app-v2/cix-manifest.json
hello from my app v2
{"cixManifest":0,"start":["message"]}
```

```sh
publisher $ cix tag "$(nix store add my-app-v2)" my-app:v1
```

```sh
consumer $ cix pull
updated 1 tag(s)
```

```sh
consumer $ cix ls -l
REF        SYSTEMS       PATH                                                   UPSTREAM         AGE
my-app:v1  x86_64-linux  /nix/store/…-my-app-v2  127.0.0.1:8420  0s
```

GC follows those pins: after the refresh, the consumer roots the new path rather than the old one.


---

[← Previous](01-hello-composix.html) · [Tour index](index.html) · [Next →](03-build-run-debug.html)
