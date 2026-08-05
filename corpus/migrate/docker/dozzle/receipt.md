# dozzle migration receipt

2026-08-05, generated `migrate.md@f474d3f` (gpt-5.6-luna). Docker mode and live socket runtime were intentionally not run.

`timeout 300 target/debug/cix build --update-lock web .#dozzle` did not finish `pnpm fetch --ignore-scripts` within the synchronous five-minute bound (exit 124). The bridge declaration itself formats/parses; no item is claimed.

The daemonless acceptance is the static declaration: `DIR /var/run/docker.sock:ro`; a real host activation additionally requires `--dir /var/run/docker.sock=host:/var/run/docker.sock --identity <stable-identity>`.
