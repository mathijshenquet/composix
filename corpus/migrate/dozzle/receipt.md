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
population chooses a different valid tile subset, and Cix pins that incidental
cache tree byte-for-byte. Forced lock refresh also reaches the historical missing
`shared_cert.pem` source failure. Independently, no faithful Cix runtime exists:
Dozzle requires Docker's host socket/API, which remains an explicit boundary.
