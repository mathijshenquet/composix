# echo-server migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (`IMPORT`, builder `ENV`, bare builder commands, explicit Node `bin/`, `CLAIM jit`, and D58's `/usr/bin/env` skeleton alias).

Docker side: historical 2026-07-30 receipt, not rerun. Historical image ID: `sha256:617137dd0795830b72301249dfbebacb2255fc8614e7eb6952f5ce6c61c53a8d`.

## `./check.sh cix`

```text
npm ci --offline --ignore-scripts --no-audit --no-fund
> echo-server@1.0.0 build
> webpack --config webpack.config.js
webpack 5.90.1 compiled with 8 warnings
PASS cix
```

Exit status: zero. `IMPORT ${pkgs.coreutils}` supplies `/bin/env`, so the fixed
`/usr/bin/env → /bin/env` sandbox alias launches webpack's generated wrapper. The
webpack warnings are upstream bundler warnings; the built service passed the bounded HTTP
probe. Final item: `/nix/store/mjvw61rg51b8zv3qvmz81n2rhphnn6is-cix-item-echo-server`.
