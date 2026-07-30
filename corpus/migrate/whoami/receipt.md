# whoami migration receipt

Date: 2026-07-30

Docker image digest: `sha256:bf3c544f03d387bd30e9b8bc2e08bc6b6f4aae80d884822fe43e472844ab5d44`

Cix item store path: `/nix/store/z7ad7rdyi4wcpbcx37rnynf4k00zvsfi-cix-item-whoami`

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
FETCH src memo hit 0ca21c0c8d15 -> /nix/store/av30i6l84v4f5y88hy2xkzkh31cmbmzy-cix-build-snapshot
BUILDER build step 1 COPY /nix/store/av30i6l84v4f5y88hy2xkzkh31cmbmzy-cix-build-snapshot/ -> . snapshot /nix/store/av30i6l84v4f5y88hy2xkzkh31cmbmzy-cix-build-snapshot
BUILDER build step 2 RUN memo hit 809f558ddf42 -> /nix/store/rcsvk7f1va6x0vplrs9rp78ppyppfy11-cix-build-snapshot
/nix/store/z7ad7rdyi4wcpbcx37rnynf4k00zvsfi-cix-item-whoami
cix unit cix-run-whoami-18c7280e51856f840.service
Hostname: ageq-devbeast
RemoteAddr: 127.0.0.1:55058
GET / HTTP/1.1
Host: 127.0.0.1
User-Agent: curl/8.14.1
Accept: */*

PASS cix
```
