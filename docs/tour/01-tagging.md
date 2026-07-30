# Tagging a build

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

Nix produced a store path. Give that immutable build a memorable local name.

```sh
$ echo 'hello from my app v1' > my-app-v1 && cix tag "$(nix store add my-app-v1)" my-app:v1
```

```sh
$ cix ls -l
REF        SYSTEMS       PATH                                                   UPSTREAM  AGE
my-app:v1  x86_64-linux  /nix/store/…-my-app-v1  -         0s
```

A name points at one immutable tag table. Cix roots that table and the store paths it currently references.

```sh
$ ls "$CIX_STATE_DIR/roots/names"
bXktYXBw
```

```sh
$ readlink "$CIX_STATE_DIR/roots/names/bXktYXBw/table"
/nix/store/…-table
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
      "narHash": "sha256-UjgGe265G0pyovh3lkIj92mKGv7d64Q9nd9w14qBQ4I=",
      "meta": {
        "reference": "my-app:v1",
        "outputs": {
          "x86_64-linux": {
            "storePath": "/nix/store/…-my-app-v1",
            "narHash": "sha256-UjgGe265G0pyovh3lkIj92mKGv7d64Q9nd9w14qBQ4I="
          }
        },
        "createdAt": "1700000000"
      }
    }
  }
}
```


---

[Tour index](index.html) · [Next →](02-moving.html)
