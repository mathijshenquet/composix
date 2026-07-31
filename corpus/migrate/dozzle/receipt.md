# dozzle migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (`IMPORT`, builder `ENV`, and `STATEDIR`).

Docker side: historical 2026-07-30 receipt, not rerun. The Docker runtime contract includes `/var/run/docker.sock`.

## `./check.sh cix`

```text
Error: line 10: FETCH output changed
  | "  FETCH pnpm fetch --ignore-scripts"
```

Exit status: non-zero. The currently pinned UI cache is also unstable. The
requested independent two-run `go mod download` reproduction found the precise
backend cause: the common 33,862 files were byte-identical, while run 1 retained
seven extra Go checksum-database tile files totaling 35,808 bytes:

```text
8192  5ce1866f...  tile/8/0/x227/780
2624  04139b3c...  tile/8/0/x227/780.p/82
3456  128cf09f...  tile/8/0/x228/452.p/108
8192  9fb9ad92...  tile/8/1/889
6272  7cb74e70...  tile/8/1/889.p/196
3200  483bc44b...  tile/8/1/892.p/100
3872  7372f718...  tile/8/2/003.p/121
```

Thus the mismatch is not changed module source: concurrent sumdb cache
population chooses a different valid tile subset. A disposable backend-only D69
reproduction (the exact source revision, a minimal generated `dist`, and
placeholder absent cert files solely to make the historical source compile) ran
`cix build --update-lock build .` twice, then `cix build .`. Both probes produced
`/nix/store/l8l5vlf0v4zzhz0vv7xrimqm7la36l3d-cix-item-proof`; the second pair's
ordinary build memo-hit. Its automatic pin is only `dozzle`, while the recorded
volatile facts include the sumdb tile paths above. This proves D69 consumed-set
keying handles the Go-side FETCH without representing the full service conversion
as passing. The UI remains the honest normalization case: its pnpm/Vite-generated
consumed `dist` bytes differ across clean runs, so D69 records rather than
excludes it. Forced lock refresh also reaches the historical missing
`shared_cert.pem` source failure. Independently, no faithful Cix runtime exists:
Dozzle requires Docker's host socket/API, which remains an explicit boundary.
