# Inspecting artifacts

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

`cix inspect` defaults to stable JSON. For a tag it combines the index entry with the validated, parsed manifest from the resolved store item.

```sh
$ cix tag /nix/store/…-service-fixture inspect-demo:v1
```

```sh
$ cix inspect inspect-demo:v1
{
  "kind": "artifact",
  "reference": "inspect-demo:v1",
  "storePath": "/nix/store/…-service-fixture",
  "narHash": "sha256-z2cqcEc8tPahnIdWgisQqJNFexX/A6OmYmtRqMuTJEU=",
  "outputs": {
    "x86_64-linux": {
      "storePath": "/nix/store/…-service-fixture",
      "narHash": "sha256-z2cqcEc8tPahnIdWgisQqJNFexX/A6OmYmtRqMuTJEU="
    }
  },
  "manifest": {
    "cixManifest": 2,
    "services": {
      "tour-service": {
        "exec": [
          "bin/service"
        ],
        "mounts": null,
        "setup": null,
        "env": {},
        "ports": {},
        "listeners": {},
        "dirs": {
          "state": [
            "/var/lib/tour-service"
          ],
          "cache": [],
          "logs": [],
          "config": [],
          "run": null
        },
        "health": null,
        "network": null,
        "jit": null
      }
    }
  },
  "closureSize": 872,
  "trustedKeys": [],
  "upstream": null,
  "drvPath": null
}
```

The entry retains per-system output slots while the selected store path supplies the manifest and Nix closure measurement. `cix inspect --human inspect-demo:v1` is the compact operator view; a live unit is selected by its exact name or unique running service name.


---

[← Previous](12-building-with-run.html) · [Tour index](index.html)
