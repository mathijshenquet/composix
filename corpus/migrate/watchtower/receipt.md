# watchtower migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (`IMPORT`, builder `ENV`, and bare `START` from artifact `bin/`).

Docker side: historical 2026-07-30 receipt, not rerun. The historical Docker build lacked the CI-provided `./watchtower` binary; its runtime contract also requires `/var/run/docker.sock`.

## `./check.sh cix`

```text
cix item /nix/store/49xrwxb2760x3g7zg47n61s6jvg4l3b0-cix-item-watchtower
```

Exit status: non-zero. The Go build succeeds, but no faithful Cix runtime conversion exists: Watchtower needs Docker's host socket/API, an explicit ❌ boundary.

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Worker evidence: a fresh staged ordinary build exited 0 with
`/nix/store/w5mwq2s06lg70k8qyz5c9bnil7d61xw8-cix-item-watchtower`; its cold
audit exited 1 at a Go build-cache read-set mismatch, and the supplied probe
exited 4 after cleanup could not find the unit.

After `bash corpus/migrate/fetch.sh watchtower` exited 0, the assembler's
ordinary build fetched Go modules, compiled the recorded revision, and exited 0
with the same item. The supplied probe memo-hit that item and again exited 4:
`Failed to stop ... Unit ... not loaded.` The explicit cold build replayed the
pinned module snapshot, then exited 1 because a `.cache/go-build/...-a` read was
warm-absent and missing from the cold observation. Docker mode was not rerun;
the absent CI-provided binary and refused Docker socket remain explicit.
