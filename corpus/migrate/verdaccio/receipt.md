# Receipt

Verdict: **build-fail** (Cixfile class: build). The conversion pins both the Corepack and pnpm dependency fetches, but did not yield a final cix item in this round.

Verbatim failing transcript from the initial build attempt:
```
$ ../../../target/debug/cix build .
[ERROR] Failed to switch pnpm to v11.1.2. Looks like pnpm CLI is missing at "/work/.pnpm-store/v11/links/@/pnpm/11.1.2/d5d049f82626f807048f13466af21737098073e057ec189edfd512367b5366a6/bin" or is incorrect
Error: line 8: FETCH failed
```
Docker digest and cix store path: not produced.

## Corpus fetch verification (2026-07-31)

The raw pinned checkout contains six development/example paths that were absent
from the historic build context. `SOURCE` now names those exclusions; the selected
checkout diffed byte-identically with the vendored tree.
