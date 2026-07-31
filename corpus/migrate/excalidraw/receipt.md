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
