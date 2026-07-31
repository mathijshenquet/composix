# filestash migration receipt

Source revision: `cdcb9566d4d24c065e461b1c8e3220ff68ef98ac` (2026-07-31).

Docker: `./check.sh docker` passed on 2026-07-31. Image:
`sha256:c5bfe487160aa1eb52d3b1afb84ebea8c403e782119ff1594329de43cb54ab51`.
The natural `/` endpoint returned its `X-Powered-By: Filestash` product header.
The upstream Dockerfile clones moving `master` rather than using the supplied
context, which is an upstream reproducibility weakness despite this pair's
recorded SOURCE pin.

Cix: build-fail. Networked Go generation had to join dependency download in
`FETCH`; its generated cache snapshot then changed between runs. After one
explicit lock acceptance, compilation reached final cgo linking. Normal nixpkgs
outputs do not ship the static archives hard-coded by Filestash's cgo directives.
Selecting the available `pkgsStatic` packages cannot close the gap either: the
locked nixpkgs `giflib` static derivation itself fails while producing a shared
object, which cancels static `libwebp` and the rest of the closure. No Cix runtime
pass is claimed. Classification: language/package-selection gap, plus a product
FETCH-stability gap.

The attempted runtime maps `FILESTASH_PATH` to `STATEDIR`, seeds the default
configuration idempotently, and explicitly carries ffmpeg/poppler helpers.
