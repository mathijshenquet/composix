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
my-app:v1	systems=x86_64-linux	path=/nix/store/…-my-app-v1	upstream=-	age=0s
```

The tag database is an `ls`-able symlink farm. Each symlink is a Nix GC root, so the pin *is* the name.

```sh
$ ls "$CIX_STATE_DIR/roots"
bXktYXBwOnYx
```

```sh
$ readlink "$CIX_STATE_DIR/roots/bXktYXBwOnYx"
/nix/store/…-my-app-v1
```

```sh
$ cat "$CIX_STATE_DIR/tags/bXktYXBwOnYx.json"
{
  "reference": "my-app:v1",
  "outputs": {
    "x86_64-linux": {
      "storePath": "/nix/store/…-my-app-v1",
      "narHash": "sha256-UjgGe265G0pyovh3lkIj92mKGv7d64Q9nd9w14qBQ4I="
    }
  },
  "createdAt": "1700000000"
}
```


---

[Tour index](index.html) · [Next →](02-moving.html)
