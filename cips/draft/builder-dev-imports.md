# builder-dev-imports — headers, libraries, and pkg-config through IMPORT

Status: **withdrawn** (2026-08-04, same day: Mathijs's review — CIP-88(a)
already decided and built this. The vendored dev-env snapshot synthesizes
the toolchain environment from the IMPORTed packages via nixpkgs' own
stdenv machinery (`nix print-dev-env`), lock-pinned; a cix-side
`IMPORT DEV` union would re-mint exactly the per-variable list the
CIP-88 amendment rejected. Filestash's hand-wired preamble predates
CIP-88 — its GAPS.md now routes to stale-regenerate, and the
regeneration is the verification that the snapshot covers the
`CPATH`/`LIBRARY_PATH` case.)

## 1. The problem

`IMPORT` unions only `bin`, `etc`, and `share`. A builder that compiles
against C dependencies must therefore hand-wire the toolchain search paths.
Filestash's build step today:

```dockerfile
RUN CPATH=${pkgs.brotli.dev}/include:${pkgs.libjpeg.dev}/include:… \
    LIBRARY_PATH=${pkgs.giflib}/lib:${pkgs.pkgsStatic.libwebp}/lib:… \
    PKG_CONFIG_PATH=${pkgs.ffmpeg.dev}/lib/pkgconfig:… \
    go build --tags fts5 -o dist/filestash cmd/main.go
```

Mathijs's review: this SHOULD be unnecessary — IMPORT exists so that tool
availability is declared, and a page-wide env preamble is exactly the
Docker-shaped noise the language dissolves elsewhere.

## 2. Prior work

Nix itself solves this with setup-hook machinery (`buildInputs` populate
`NIX_CFLAGS_COMPILE`/`NIX_LDFLAGS` via the stdenv). Composix builders are
deliberately simpler — a bare workdir plus IMPORT — so the equivalent
surface has to be an explicit language feature, not ambient stdenv magic.
Any native-code migration (filestash is the corpus's L-effort case) hits
this wall; pure script/JVM/Go-without-cgo cases never do.

## 3. Recommendation

Three options considered:

- **(a) Plain IMPORT grows the dev surface**: also union `include/`, `lib/`,
  `lib/pkgconfig/` when present, and default `CPATH=/include`,
  `LIBRARY_PATH=/lib`, `PKG_CONFIG_PATH=/lib/pkgconfig` (overridable by
  explicit `ENV`). Just Works, but IMPORT stops being a trivially
  describable bin/etc/share union and earlier-wins starts to matter for
  linkage correctness.
- **(b) A distinct directive** — `IMPORT DEV ${…}` (or `DEVIMPORT`) doing
  (a)'s union + env exports, plain IMPORT untouched. More grammar, but the
  builder reads honestly: tools vs toolchain inputs are visually distinct.
- **(c) Status quo + documented pattern**: canonical `ENV CPATH = …` lines
  above the RUN in migrate.md. Cheapest; keeps the noise.

Leaning **(b)**: the dev surface is a genuinely different contract (search
paths for a compiler, not executables on PATH), and an explicit keyword
keeps plain IMPORT's semantics trivially describable.

## 4. Open questions

- Keyword bikeshed: `IMPORT DEV` vs `DEVIMPORT` vs `IMPORT --dev`?
- Do the exported env defaults appear in the build key even when unused?
- Should `pkg-config` itself be auto-imported when a `pkgconfig/` tree is
  present, or stay an explicit `IMPORT ${pkgs.pkg-config}`?
- Does (b) subsume the multi-output story (`.dev`/`.out` selection stays the
  author's job via explicit output references)?
