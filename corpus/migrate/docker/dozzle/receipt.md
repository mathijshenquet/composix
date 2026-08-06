# dozzle migration receipt

2026-08-05, generated `migrate.md@f474d3f` (gpt-5.6-luna). Docker mode and live socket runtime were intentionally not run.

`timeout 300 target/debug/cix build --update-lock web .#dozzle` did not finish `pnpm fetch --ignore-scripts` within the synchronous five-minute bound (exit 124). The bridge declaration itself formats/parses; no item is claimed.

The daemonless acceptance is the static declaration: `DIR /var/run/docker.sock:ro`; a real host activation additionally requires `--dir /var/run/docker.sock=host:/var/run/docker.sock --identity <stable-identity>`.

## 2026-08-06 pnpm-wall diagnosis

An actual cix IMPORT-union A/B diagnosed the old timeout. Without
`${pkgs.cacert}`, a clean `timeout 180 target/debug/cix build ...` exited 124
with no store files; `/var/tmp/cix-read-trace-pSTPmk/syscalls` ended in
repeated failed OpenSSL certificate-directory lookups. With cacert imported,
the same FETCH completed both probes and exited 0, producing the CAS-only
item `/nix/store/rb7n26wc79a6bqypsyjv95ag9rpkgr43-cix-item-pnpm-store`.
The foreground log is
`/var/tmp/cix-pnpmwall-cix-with-green.8niV1O/build.log`.

A separate exact-pnpm-11.17.0 network probe exited 0 in 6561 ms, verified 818
lock entries, recorded 52 IPv6 and 38 IPv4 connects, and saw every package on
fetch attempt 1 with no TLS errors. The old symptom was a cacert masquerade,
not IPv6 fallback. This receipt claims only the FETCH diagnosis; the full
frontend and runtime were not rerun.

## 2026-08-06 frozenStore route

The translation retains upstream's already-eligible pnpm 11.17.0 contract by
selecting pinned nixpkgs pnpm 11.18.0, fetches the complete store, and points a
separate offline builder at that immutable instance with `frozen-store=true`.
The actual Cix update probe completed both fetches in 15,041 ms, verified all
818 lock entries, and reported volatility only in pnpm verification metadata,
`.modules.yaml`, and `v11/index.db`. It sealed 20,175 CAS files plus the
read-only index at
`/nix/store/44apdds69qhw5gr23cby7416mm1m09xx-cix-build-consumed` (NAR hash
`sha256-u2dxDDHhs2yvNCov2ychAQ1Vn4NlzX63ouCFjTZb4PU=`).

The downstream offline frozen install then remained active under Cix's read
tracer until the foreground command synchronously exited 124 at its
1,200-second bound. No pnpm/store error was emitted. This is a trace-cost wall,
not a failed store seal or a green frontend/item/runtime claim. Exact evidence
is `/var/tmp/cix-pnpm-frozenstore-dozzle-route.MqWXUV`; no lock change is
retained.
