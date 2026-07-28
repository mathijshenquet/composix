# Tagging a build

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

Nix produced a store path. Give that immutable build a memorable local name.

```sh
$ mkdir -p fixture-v1 && printf '%s\n' 'hello from my app v1' > fixture-v1/README
```

```sh
$ nix store add-path fixture-v1
/nix/store/…-fixture-v1
```

```sh
$ cix tag /nix/store/…-fixture-v1 my-app:v1
```

```sh
$ cix ls -l
my-app:v1	systems=x86_64-linux	path=/nix/store/…-fixture-v1	upstream=-	age=0s
```

The tag database is an `ls`-able symlink farm. Each symlink is a Nix GC root, so the pin *is* the name.

```sh
$ ls "$CIX_STATE_DIR/roots"
bXktYXBwOnYx
```

```sh
$ readlink "$CIX_STATE_DIR/roots/bXktYXBwOnYx"
/nix/store/…-fixture-v1
```

```sh
$ cat "$CIX_STATE_DIR/tags/bXktYXBwOnYx.json"
{
  "reference": "my-app:v1",
  "outputs": {
    "x86_64-linux": {
      "storePath": "/nix/store/…-fixture-v1",
      "narHash": "sha256-v5Zwn2my2NZ+aU6i3A6Bc2qiKIIrl34dVATfFhhnIZ8="
    }
  },
  "createdAt": "1700000000"
}
```


---

[Tour index](index.html) · [Next →](02-moving.html)
