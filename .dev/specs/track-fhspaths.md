# track/fhspaths — CIP-95 implementation: the FHS path surface (build-first)

Read AGENTS.md first (gate convention; synchronous receipts), then
cips/accepted/0095-fhs-paths.md — the contract; its Decision section
binds. Work in the herdr worktree on branch `track/fhspaths`. Keep
`crates/cix-build/LOG.md` current (dated track heading; commit it).

## Phase 1 — the ld.so wiring spike (report before phase 2)

Question: with the loader alias in place, how do DT_NEEDED libraries
resolve from the `/lib` union WITHOUT breaking nix-built binaries in the
same builder?

Build a hermetic repro: construct an FHS-linked ELF deliberately
(compile a small C program in one builder, `patchelf --set-interpreter
/lib64/ld-linux-x86-64.so.2` and strip its RPATH so it depends on
`libc.so.6` resolution), then in a fresh builder with only
`IMPORT ${pkgs.glibc}` + the prototype surface, exec it. Verify:

1. PT_INTERP resolves through the alias (kernel side).
2. `/lib` search wiring candidates, measured: a generated
   `/etc/ld.so.conf` + ldconfig cache in the union, vs a default
   library path. Naive `LD_LIBRARY_PATH=/lib` is suspect (outranks
   DT_RUNPATH); whatever you pick, prove a nix-built RUNPATH-carrying
   binary in the same builder still resolves its own closure unshadowed
   (that's the load-bearing assertion — write it as a test).
3. The musl alias variant with a musl-linked test ELF.

Spike verdict in the LOG. If the wiring cannot be made clean, STOP and
report — CIP-95 names the fallback; do not improvise it.

## Phase 2 — implementation (spike clean)

- Skeleton loader aliases: x86_64 GNU + musl now (grow-on-demand per
  the CIP), dangling-until-imported like `/usr/bin/env`, versioned into
  the skeleton constant.
- `lib/` joins IMPORT's union (earlier-wins). Builders first; then
  measure artifact-side closure impact on a representative item and
  either enable artifacts too (the CIP leans yes for runtime spawns) or
  report the measured cost and leave it staged — your call with numbers
  in the LOG.
- (E) diagnostics: on a failed RUN step, surface execve-ENOENT inside
  the workdir, FHS-loader reads, and failed SONAME opens from the
  existing trace parse; match against imported packages + the
  loader→libc map; hint the IMPORT (mirror the /usr/bin/env hint
  style). Retention of negative trace results only as far as the
  failing step's report needs.
- Acceptance: your constructed FHS-ELF case green with zero patchelf
  lines; additionally reproduce the fhsspike sass-embedded evidence
  (branch track/fhsspike has the Cixfile) far enough to show the loader
  failure is gone — the full directus build still hits the separate
  bare `Error: Not a directory` defect, which is OUT of scope; do not
  fix it here, but if your work happens to reveal its cause, record it.
- Docs: teach the surface in docs/migrate.md (the FHS/native-binary
  row + the taught patchelf escape per CIP-95 §(A)). docs/cixfile.md is
  being rewritten concurrently by track/cixdocs — merge origin/main
  before touching it, and keep your addition to one focused section.
- Ledger currency: directus GAPS flips to
  `Status: stale — regenerate with CIP-95`; grep for other exhibiting
  cases.

FENCE: track/cixdocs (docs/cixfile.md) runs concurrently — merge before
docs edits as above. Do not touch corpus Cixfiles, cips/ content, or
compose/netns/health code.

## Gate

Standard agent tier plus the focused VM scenarios your skeleton change
touches (the skeleton is load-bearing for every builder: run at least
one build-heavy scenario and the closed-root audit focused). Bounded.
Synchronous receipts with exact repro commands.
