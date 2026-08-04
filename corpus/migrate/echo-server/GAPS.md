Generated: migrate.md@e1978b6 · gpt-5.6-luna · 2026-08-04
Status: current

- `install-dependencies.sh` is conversion-owned build machinery used to isolate npm's cache and logs; it is not part of the upstream deploy unit. → case
- The moving `node:lts-alpine` image is represented by locked nixpkgs Node rather than an exact Node/Alpine image identity. → case
- The direct dependency FETCH is cold-divergent because warm execution observes `node_modules` as a directory while a cold workspace observes it absent; this should be diagnosed and normalized by [CIP-87's cold-divergence machinery](../../../cips/accepted/0087-read-set-keying.md). → language (cold divergence audit)
- The worker's warm build/probe passed, but the independent fresh fetch is EXPECT-hostile (`sha256-MFqh…` declared versus `sha256-NV8V…` fetched), so no cold replay snapshot exists; keep the pin unchanged and normalize the dependency output in the volatile-fetch fix round. → case (cold stability)
