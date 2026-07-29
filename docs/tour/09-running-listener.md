# Running with a listener

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

A spec-v3 listener gives the service an already-bound socket, so the process has no authority to create another network socket.

```sh
$ cix run /nix/store/…-listener-fixture --user -p http=127.0.0.1:8420 --detach
cix-run-listenfds-NONCE.service
warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path
warning: user manager rejected sandbox controls; degraded fallback required
warning: retrying with degraded sandbox controls (D13)
```

```sh
$ curl -fsS http://127.0.0.1:8420
LISTEN_FDS=1; no socket() authority
```

```sh
$ systemctl --user stop cix-run-listenfds-NONCE.service
```

The user-manager path is suitable for rootless development; production uses the system manager. Stopping the transient service also removes its companion `.socket` unit.


---

[← Previous](08-building-cixfile.html) · [Tour index](index.html) · [Next →](10-composing-services.html)
