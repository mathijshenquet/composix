# whoami migration receipt

Date: 2026-07-30
Refreshed: 2026-07-30 (D50–D53 syntax; builder-local multi-FETCH pins)

Docker image digest: `sha256:bf3c544f03d387bd30e9b8bc2e08bc6b6f4aae80d884822fe43e472844ab5d44`

Cix item store path: `/nix/store/y696s2gxr34bvcqzndm8gz2hkkhf9fci-cix-item-whoami`

## `./check.sh docker`

```text
docker image sha256:bf3c544f03d387bd30e9b8bc2e08bc6b6f4aae80d884822fe43e472844ab5d44
Hostname: 4bcf34c4b535
IP: 127.0.0.1
IP: ::1
IP: 172.17.0.2
RemoteAddr: 172.17.0.1:51182
GET / HTTP/1.1
Host: 127.0.0.1:18080
User-Agent: curl/8.14.1
Accept: */*

PASS docker
```

## `./check.sh cix`

```text
BUILDER build step 1 FETCH memo hit adf7dc34e8d3 -> /nix/store/53gfms5rg2mi1azv3d7i9jyh0plmgsp0-cix-build-snapshot
BUILDER build step 2 FETCH memo hit 73b409fc87bb -> /nix/store/r4hwf34z2ahq6piljcmrxr0sav5qs53m-cix-build-snapshot
BUILDER build step 3 RUN memo hit 7f7350b70bbc -> /nix/store/7l36h0badcgz22acl0n8c3s2mb2ppxx4-cix-build-snapshot
/nix/store/y696s2gxr34bvcqzndm8gz2hkkhf9fci-cix-item-whoami
cix unit cix-run-whoami-18c7312f6dddcbdb0.service
Hostname: ageq-devbeast
RemoteAddr: 127.0.0.1:51264
GET / HTTP/1.1
Host: 127.0.0.1
User-Agent: curl/8.14.1
Accept: */*

PASS cix
```
