# verdaccio migration receipt

2026-08-05, generated `migrate.md@f474d3f` (gpt-5.6-luna). Docker mode was not rerun.

Independent ordinary builds installed the 38-workspace pnpm graph but produced no item. The cold replay exited non-zero before build/deploy:

```text
recorded read set differs between warm and cold at "."
```

The two directory hashes differed at the `FETCH pnpm install` step. This is cold volatility, not a repinning opportunity; `/-/ping` was not run.
