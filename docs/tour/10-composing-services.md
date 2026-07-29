# Composing services

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

```sh
$ cix tag /nix/store/…-compose-fixture-v1 tour-compose:current
```

Compose v0 accepts strict machine-format JSON. This self-contained item is a Nix store path added by the harness, then named with a local tag.

```sh
$ cat compose.json
{
  "composeVersion": 1,
  "name": "tour-compose",
  "services": {
    "web": {
      "item": "tour-compose:current",
      "update": "track"
    }
  }
}
```

```sh
$ cix compose check compose.json
compose tour-compose: 1 services, 0 edges, valid
```

`check` resolves and validates without activation. Root `cix up` owns the persistent lock write, so this rootless harness records the checked tag's actual resolved values in `cix.lock` before inspecting that format.

```sh
$ cat cix.lock
{
  "services": {
    "web": {
      "ref": "tour-compose:current",
      "storePath": "/nix/store/…-compose-fixture-v1",
      "narHash": "sha256-Tbi6YgEjyPHrmTOf/kxLMIFUgbRiv2gHaAnpYUm9bMo="
    }
  }
}
```

```sh
$ cix compose diff compose.json
unit added: cix-tour-compose-web.service
unit added: cix-tour-compose.slice
unit added: cix-tour-compose.target
service web: - -> /nix/store/…-compose-fixture-v1
```

```sh
$ cix tag /nix/store/…-compose-fixture-v2 tour-compose:current
```

Moving the tracked tag changes the dry-built generation without starting a service. With no root-managed active profile in this rootless scenario, `-` means there is no prior active item to compare.

```sh
$ cix compose diff compose.json
unit added: cix-tour-compose-web.service
unit added: cix-tour-compose.slice
unit added: cix-tour-compose.target
service web: - -> /nix/store/…-compose-fixture-v2
```

`cix up`, `cix rollback`, and `cix down` manage the system manager and therefore require root; the [stack example](../../examples/compose/stack/) VM check covers activation, selective update, rollback, and cleanup.


---

[← Previous](09-running-listener.html) · [Tour index](index.html)
