# memcached migration receipt

Date: 2026-07-30
Refreshed: 2026-07-30 (context/ layout and D50–D53 corpus run; unchanged Cix item)

Docker image digest (local image ID): `sha256:176adbf343271bf411648dced32bbe97b2c734052e66bfc11ba7ba7aebeea8d5`

Cix item store path: `/nix/store/kg6afp6d8dvkwjl8r3qip4yyy3y5lpww-cix-item-memcached`

## `./check.sh docker`

```text
docker image sha256:176adbf343271bf411648dced32bbe97b2c734052e66bfc11ba7ba7aebeea8d5
VERSION 1.6.45
PASS docker
```

Exit status: 0

## `./check.sh cix`

```text
/nix/store/kg6afp6d8dvkwjl8r3qip4yyy3y5lpww-cix-item-memcached
cix unit cix-run-memcached-18c731316bd213570.service
VERSION 1.6.42
PASS cix
```

Exit status: 0
