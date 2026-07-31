# dozzle migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (`IMPORT`, builder `ENV`, and `STATEDIR`).

Docker side: historical 2026-07-30 receipt, not rerun. The Docker runtime contract includes `/var/run/docker.sock`.

## `./check.sh cix`

```text
Error: line 19: FETCH output changed
  | "  FETCH go mod download"
```

Exit status: non-zero. Forced lock refresh reaches the subsequent missing `shared_cert.pem` source failure. Independently of both build failures, no faithful Cix runtime conversion exists: Dozzle requires Docker's host socket/API, which remains an explicit ❌ boundary.
