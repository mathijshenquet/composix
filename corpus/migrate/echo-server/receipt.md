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

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Worker warm evidence: the staged ordinary build and HTTP probe exited 0 with
`/nix/store/lxc1wzr8vzymj9cz6x13iqqhxdi0f6sf-cix-item-echo-server`. Its staged
cold audit exited 1 because `node_modules` was a warm directory and cold-absent.

After `bash corpus/migrate/fetch.sh echo-server` exited 0, both
`target/debug/cix build corpus/migrate/echo-server` and the build performed by
the supplied probe exited 1: dependency installation completed, then FETCH
rejected declared `sha256-MFqh9XfZ43Pa4nlE/vi0Q081ZpscMZtMTn0qv7vRubQ=`
versus fetched `sha256-NV8V74j/GO7g6Zvh7zLa6g85KCtl3kiSlDJxLlH93Qw=`.
`target/debug/cix build --cold corpus/migrate/echo-server` also exited 1 because
the failed fresh fetch left no replay snapshot. No EXPECT or lock pin was
updated. Docker mode was not rerun.
