# watchtower migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (`IMPORT`, builder `ENV`, and bare `START` from artifact `bin/`).

Docker side: historical 2026-07-30 receipt, not rerun. The historical Docker build lacked the CI-provided `./watchtower` binary; its runtime contract also requires `/var/run/docker.sock`.

## `./check.sh cix`

```text
cix item /nix/store/49xrwxb2760x3g7zg47n61s6jvg4l3b0-cix-item-watchtower
```

Exit status: non-zero. The Go build succeeds, but no faithful Cix runtime conversion exists: Watchtower needs Docker's host socket/API, an explicit ❌ boundary.
