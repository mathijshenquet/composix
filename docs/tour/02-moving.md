# Moving a tag

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

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


---

[← Previous](01-tagging.html) · [Tour index](index.html) · [Next →](03-untagging.html)
