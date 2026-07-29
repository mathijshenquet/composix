# Moving a tag

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

A tag can move to a newer build without changing the immutable store paths behind it.

```sh
$ echo 'hello from my app v1' > my-app-v1 && cix tag "$(nix store add my-app-v1)" my-app:v1
```

```sh
$ echo 'hello from my app v2' > my-app-v2 && cix tag "$(nix store add my-app-v2)" my-app:v1
```

```sh
$ cix ls -l
REF        SYSTEMS       PATH                                                   UPSTREAM  AGE
my-app:v1  x86_64-linux  /nix/store/…-my-app-v2  -         0s
```

Tags are mutable pointers over immutable store paths. Retagging changes the symlink; the old path is now unpinned by this tag.

```sh
$ readlink "$CIX_STATE_DIR/roots/bXktYXBwOnYx"
/nix/store/…-my-app-v2
```


---

[← Previous](01-tagging.html) · [Tour index](index.html) · [Next →](03-untagging.html)
