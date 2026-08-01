# Chapter 1: The index

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

The index gives mutable, memorable names to immutable Nix store paths. This chapter follows one tag through its complete local life.

## Tag

```sh
$ mkdir my-app-v1 && printf '%s\n' 'hello from my app v1' > my-app-v1/message && printf '%s\n' '{"cixManifest":0,"start":["message"]}' > my-app-v1/cix-manifest.json
```

```sh
$ ls -1 my-app-v1
cix-manifest.json
message
```

```sh
$ cat my-app-v1/message my-app-v1/cix-manifest.json
hello from my app v1
{"cixManifest":0,"start":["message"]}
```

```sh
$ cix tag "$(nix store add my-app-v1)" my-app:v1
```

```sh
$ cix ls -l
REF        SYSTEMS       PATH                                                   UPSTREAM  AGE
my-app:v1  x86_64-linux  /nix/store/…-my-app-v1  -         0s
```

The name points at an immutable tag table. Cix roots that table, which in turn keeps the store paths in its current entries alive.

```sh
$ ls "$CIX_STATE_DIR/roots/names"
bXktYXBw
```

```sh
$ cat "$(readlink $CIX_STATE_DIR/roots/names/bXktYXBw/table)/table.json"
{
  "cixTagTable": 1,
  "name": "my-app",
  "parent": null,
  "tags": {
    "v1": {
      "storePath": "/nix/store/…-my-app-v1",
      "narHash": "sha256-iUMlkbB006RUsAvCgp43+tHkb8uxbWjnGu7KbTSMo7w=",
      "meta": {
        "reference": "my-app:v1",
        "outputs": {
          "x86_64-linux": {
            "storePath": "/nix/store/…-my-app-v1",
            "narHash": "sha256-iUMlkbB006RUsAvCgp43+tHkb8uxbWjnGu7KbTSMo7w="
          }
        },
        "createdAt": "1700000000"
      }
    }
  }
}
```

## Inspect

Inspection resolves the tag, then combines its per-system index entry with the parsed runtime manifest and measured Nix closure as stable JSON.

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
      "logs": [],
      "run": null,
      "state": []
    },
    "egress": false,
    "env": {},
    "health": null,
    "jit": null,
    "listeners": {},
    "mounts": null,
    "network": null,
    "ports": {},
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

## Move

Retagging atomically moves the name to a newer immutable build. The old path does not change; this name simply stops pinning it.

```sh
$ mkdir my-app-v2 && printf '%s\n' 'hello from my app v2' > my-app-v2/message && printf '%s\n' '{"cixManifest":0,"start":["message"]}' > my-app-v2/cix-manifest.json
```

```sh
$ ls -1 my-app-v2
cix-manifest.json
message
```

```sh
$ cat my-app-v2/message my-app-v2/cix-manifest.json
hello from my app v2
{"cixManifest":0,"start":["message"]}
```

```sh
$ cix tag "$(nix store add my-app-v2)" my-app:v1
```

```sh
$ cix ls -l
REF        SYSTEMS       PATH                                                   UPSTREAM  AGE
my-app:v1  x86_64-linux  /nix/store/…-my-app-v2  -         0s
```

## Untag

Removing the tag writes a new table with no `v1` entry. The history remains inspectable in immutable predecessor tables, but fresh resolution no longer offers the tag.

```sh
$ cix untag my-app:v1
```

```sh
$ cix ls
```

The next `nix-collect-garbage` may reclaim bytes that no other root still reaches.


---

[Tour index](index.html) · [Next →](02-distribution.html)
