# Chapter 4: Naming and distribution

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

You will give an immutable item a family of operational names, move and remove those names, then serve and refresh one across an index boundary. Afterwards, you will understand why tags are GC-rooted pointers rather than build inputs and how ordinary Nix caches carry the bytes.

## One demystifying aside: an item is a tree

Normally `cix build` writes this tree for you. At the boundary, however, an item is simply a Nix store tree with `cix-manifest.json`; this is the tour's one hand-assembled example, kept short so you can see that no image format is hiding underneath.

```sh
$ mkdir my-app-v1 && printf '%s\n' 'hello from my app v1' > my-app-v1/message && printf '%s\n' '{"cixManifest":0,"start":["message"]}' > my-app-v1/cix-manifest.json
```

```sh
$ ls -1 my-app-v1
cix-manifest.json
message
```

```sh
$ cat my-app-v1/message
hello from my app v1
```

```sh
$ cix tag "$(nix store add my-app-v1)" my-app:v1
```

```sh
$ cix inspect my-app:v1
{
  "kind": "artifact",
  "reference": "my-app:v1",
  "storePath": "/nix/store/…-my-app-v1",
  "narHash": "sha256-iUMlkbB006RUsAvCgp43+tHkb8uxbWjnGu7KbTSMo7w=",
  "outputs": {
    "x86_64-linux": {
      "storePath": "/nix/store/…-my-app-v1",
      "narHash": "sha256-iUMlkbB006RUsAvCgp43+tHkb8uxbWjnGu7KbTSMo7w="
    }
  },
  "manifest": {
    "cixManifest": 0,
    "dirs": {
      "cache": [],
      "config": [],
      "data": [],
      "logs": [],
      "run": null,
      "state": []
    },
    "egress": false,
    "env": {},
    "jit": null,
    "listeners": {},
    "liveness": null,
    "mounts": null,
    "network": null,
    "ports": {},
    "readiness": null,
    "secrets": {},
    "shm": null,
    "start": [
      "message"
    ],
    "start_pre": null
  },
  "closureSize": 544,
  "trustedKeys": [],
  "upstream": null,
  "drvPath": null
}
```

## Names come after builds

The store path already has its complete identity. Tags add mutable operational vocabulary and GC roots after that build, so changing a tag never changes the item it points at. A slash groups related members into a family; the explicit suffix is always a tag, with no magic `latest`.

```sh
$ cix tag my-app:v1 guide/web:v1
```

```sh
$ cix tag my-app:v1 guide/web:stable
```

```sh
$ cix ls -l guide/
REF               SYSTEMS       PATH                                                   UPSTREAM  AGE
guide/web:stable  x86_64-linux  /nix/store/…-my-app-v1  -         0s 
guide/web:v1      x86_64-linux  /nix/store/…-my-app-v1  -         0s
```

```sh
$ cix inspect guide/web:v1 | jq '{kind, reference, storePath}'
{
  "kind": "artifact",
  "reference": "guide/web:v1",
  "storePath": "/nix/store/…-my-app-v1"
}
```

There is no mutable image object to rename or delete. Move a name by tagging the destination and untagging the source; remove one with `cix untag`. Nix garbage collection may reclaim an item only after no cix tag or other GC root reaches it.

```sh
$ cix tag guide/web:v1 guide/web:release && cix untag guide/web:stable
```

```sh
$ cix ls guide/
guide/web:release
guide/web:v1
```

Moving `guide/web:v1` to a new build changes only that pointer. The immutable v1 path still exists wherever another root retains it.

```sh
$ cix tag /nix/store/…-my-app-v2 guide/web:v1
```

```sh
$ cix ls -l guide/
REF                SYSTEMS       PATH                                                   UPSTREAM  AGE
guide/web:release  x86_64-linux  /nix/store/…-my-app-v1  -         0s 
guide/web:v1       x86_64-linux  /nix/store/…-my-app-v2  -         0s
```

## Serve and pull

`cix serve --with-store` exposes the bare local tag database and a standard Nix binary cache. One content-negotiated URL returns HTML to a browser and the index entry to a cix client; the index resolves names, while Nix signatures and NAR hashes protect content.

```sh
publisher $ cix serve --with-store --listen 127.0.0.1:8420 &
```

```sh
publisher $ curl -s -H 'Accept: application/vnd.cix+json;version=1' http://127.0.0.1:8420/guide/web:v1 | jq '{outputs, substituters}'
{
  "outputs": {
    "x86_64-linux": {
      "storePath": "/nix/store/…-my-app-v2",
      "narHash": "sha256-KXXUskqxQDjPHhzCKBZjWcvnl6wuoCuV0/Q0pnNHcBQ="
    }
  },
  "substituters": [
    "http://127.0.0.1:8420/store"
  ]
}
```

A qualified ref carries its origin. `--as` adopts it under a bare local name while remembering that upstream, and a later bare `cix pull` refreshes every such moving tag.

```sh
consumer $ cix pull 127.0.0.1:8420/guide/web:v1 --as guide/web:v1
updated 1 tag(s)
```

```sh
consumer $ cix ls -l
REF           SYSTEMS       PATH                                                   UPSTREAM         AGE
guide/web:v1  x86_64-linux  /nix/store/…-my-app-v2  127.0.0.1:8420  0s
```

```sh
publisher $ cix tag /nix/store/…-my-app-v3 guide/web:v1
```

```sh
consumer $ cix pull
updated 1 tag(s)
```

```sh
consumer $ cix ls -l
REF           SYSTEMS       PATH                                                   UPSTREAM         AGE
guide/web:v1  x86_64-linux  /nix/store/…-my-app-v3  127.0.0.1:8420  0s
```

The result is deliberately small: mutable HTTP names select immutable store paths, and standard Nix substitution moves their closures. No daemon-owned image graph or default registry is required.


---

[← Previous](03-building.html) · [Tour index](index.html) · [Next →](05-runtime-contract.html)
