# caddy migration receipt

Date: 2026-07-30

Docker image digest (local image ID): `sha256:bc898ebca91b534678f81a7d6d352a788be403e3e7af027564731f9dd77fa9c4`

Cix item store path: `/nix/store/vbs25dg9r93zngciqwnwapfgjfa5ivpm-cix-item-caddy`

## `./check.sh docker`

```text
docker image sha256:bc898ebca91b534678f81a7d6d352a788be403e3e7af027564731f9dd77fa9c4
PASS docker
```

Exit status: 0

## `./check.sh cix`

```text
/nix/store/vbs25dg9r93zngciqwnwapfgjfa5ivpm-cix-item-caddy
cix unit cix-run-caddy-18c72a1ab1cd3d840.service
```

Exit status: 1 (the bounded HTTP probe did not succeed).
