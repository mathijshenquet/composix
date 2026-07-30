# nats migration receipt

Date: 2026-07-30
Refreshed: 2026-07-30 (D50–D53 corpus run; unchanged Cix item)

Docker image digest: `sha256:f0f977e50ad69c0b9a041f145cce27df06166295792391f98f4ac415a067756c`

Cix item store path: `/nix/store/x0q9whg4ff6khpr23lkmlz5bzlpqjiz6-cix-item-nats`

## `./check.sh docker`

```text
docker image sha256:f0f977e50ad69c0b9a041f145cce27df06166295792391f98f4ac415a067756c
{"status":"ok"}PASS docker
```

## `./check.sh cix`

```text
/nix/store/x0q9whg4ff6khpr23lkmlz5bzlpqjiz6-cix-item-nats
cix unit cix-run-nats-18c7313166451adb0.service
{"status":"ok"}PASS cix
```
