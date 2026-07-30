# adminer migration receipt

Date: 2026-07-30

Docker image digest (local image ID): `sha256:1b74a51d52b661e95107ce2eaec2186d2f55c7c2d5d73602b76a0e6897778659`

Cix item store path: `/nix/store/6wqrprc1lqkb7g116812x0d4wvkfx17p-cix-item-adminer`

## `./check.sh docker`

```text
docker image sha256:1b74a51d52b661e95107ce2eaec2186d2f55c7c2d5d73602b76a0e6897778659
PASS docker
```

Exit status: 0

## `./check.sh cix`

```text
adminer.php: OK
FETCH adminer_src memo miss ed6c877a4f3f (98 ms) -> /nix/store/avgkp0gxdl3ib2lxd5khs7vg24sxbh3a-cix-build-snapshot
/nix/store/6wqrprc1lqkb7g116812x0d4wvkfx17p-cix-item-adminer
cix unit cix-run-adminer-18c72a16151087fa0.service
PASS cix
```

Exit status: 0
