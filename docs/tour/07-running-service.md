# Running a service

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

A spec'd store item can become a transient systemd service without root.

```sh
$ cix run /nix/store/…-service-fixture --detach --user
cix-run-tour-service-NONCE.service
warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target
warning: the user manager rejected mount-namespace sandboxing (host-specific diagnostic)
warning: retrying without PrivateUsers, ProtectSystem, ProtectHome, PrivateTmp, and BindPaths; managed *Directory persistence remains, but declared app paths will not be remapped
```

`--user` is the rootless development mode. The product target is the system manager, with `DynamicUser` and the full hardening profile; see the [design document](../design.html). The VM check exercises that system path.

```sh
$ cix ps
MANAGER  UNIT                                            STATE       DESCRIPTION
user     cix-run-tour-service-NONCE.service  active/running  [systemd-run] /nix/store/…-service-fixture/bin/service
user     cix-run.slice                                   active/active  Slice /cix/run
```

```sh
$ systemctl --user stop cix-run-tour-service-NONCE.service
```

```sh
$ cix ps
MANAGER  UNIT           STATE       DESCRIPTION
user     cix-run.slice  active/active  Slice /cix/run
```

The unit disappears once stopped; its managed state directory follows the user-manager lifecycle.


---

[← Previous](06-pull-follows.html) · [Tour index](index.html)
