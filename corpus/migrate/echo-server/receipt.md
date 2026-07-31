# echo-server migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (`IMPORT`, builder `ENV`, bare builder commands, explicit Node `bin/`, and `GRANT jit`).

Docker side: historical 2026-07-30 receipt, not rerun. Historical image ID: `sha256:617137dd0795830b72301249dfbebacb2255fc8614e7eb6952f5ce6c61c53a8d`.

## `./check.sh cix`

```text
npm ci --ignore-scripts --no-audit --no-fund
added 435 packages in 1s
sh: /work/node_modules/.bin/webpack: /usr/bin/env: bad interpreter: No such file or directory
Error: line 8: RUN failed
```

Exit status: non-zero; no item was produced. This remains a non-passing npm-build row; no retry workaround from migrate-r5 was introduced.
