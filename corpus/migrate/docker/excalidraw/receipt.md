# excalidraw migration receipt

Source revision: `786ab266ff3a9cfffaed16804cf9132b44bc08ae` (2026-07-31).

Docker: `./check.sh docker` passed on 2026-07-31. Image:
`sha256:6b5d15281cc14f4a9f8a1f3b4323171c899863b1a6c45173135c68232b7cddd3`.
The `/` response contained `Excalidraw Whiteboard`.

Cix: `./check.sh cix` passed. Item:
`/nix/store/7y17li69l72zz85lkp7bpka7bgnb7dqy-cix-item-excalidraw`. The
same title probe passed after the host's documented D36 PrivatePIDs fallback.

The Docker `HEALTHCHECK` probe is preserved by `check.sh`, but Cixfile cannot yet
encode D48 health edges; even if the paired HTTP probe passes, that missing
declarative health contract remains a product gap.

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Worker evidence: the staged ordinary and cold builds exited 0 with
`/nix/store/6bc9grm0jqgab6mkxkybyidxg9bagpw5-cix-item-excalidraw`. The exact
supplied Cix probe exited 1 because systemd could not execute the repository
runner in its namespace (`203/EXEC`); an in-store runner independently proved
the service served `Excalidraw Whiteboard` on its faithful port 80. The harness
probes 18090 without passing a port override.

After `bash corpus/migrate/docker/fetch.sh excalidraw` exited 0, the assembler's
`target/debug/cix build corpus/migrate/docker/excalidraw` executed Yarn/Vite and exited
0 with the same item. The supplied probe again exited 1 at the runner-path
`203/EXEC` boundary. `target/debug/cix build --cold corpus/migrate/docker/excalidraw`
replayed the pinned FETCH snapshot and exited 0 with the same item. Docker mode
was not rerun.
