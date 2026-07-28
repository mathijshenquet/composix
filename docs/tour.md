# cix — local index tour

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

This five-minute tour covers local tags only: naming a build, moving that name, and removing it.

## Tagging a build

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


## Moving a tag

A tag can move to a newer build without changing the immutable store paths behind it.

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
$ mkdir -p fixture-v2 && printf '%s\n' 'hello from my app v2' > fixture-v2/README
```

```sh
$ nix store add-path fixture-v2
/nix/store/…-fixture-v2
```

```sh
$ cix tag /nix/store/…-fixture-v2 my-app:v1
```

```sh
$ cix ls -l
my-app:v1	systems=x86_64-linux	path=/nix/store/…-fixture-v2	upstream=-	age=0s
```

Tags are mutable pointers over immutable store paths. Retagging changes the symlink; the old path is now unpinned by this tag.

```sh
$ readlink "$CIX_STATE_DIR/roots/bXktYXBwOnYx"
/nix/store/…-fixture-v2
```


## Untagging

Removing a tag removes its local GC root and its metadata sidecar.

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
$ cix untag my-app:v1
```

```sh
$ cix ls
```

Unpinned means the next `nix-collect-garbage` may reclaim the build; nothing else in cix holds it.

