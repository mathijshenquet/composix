# track/inspect — cix inspect (D35d) + systems column in cix ls -l (D35e)

Read AGENTS.md first. Authoritative design: docs/design.md **D35 (d)** — implement exactly
that surface; where detail is missing, follow the ledger rows in docs/docker.md
(`image inspect`, `container inspect`) and existing CLI idioms in crates/cix.

## cix inspect

One verb, two worlds, dispatched on the argument:

- `cix inspect <ref-or-installable>` (artifact): store path, narHash, per-system outputs
  (from the index entry when the target is a tag; for a bare store path, the current
  system only), the resolved manifest (parsed, not raw bytes — reuse the cix-run parsing),
  closure size (`nix path-info -S` equivalent; nix-free fallback may print "unavailable"),
  signatures/keys if recorded in the entry sidecar, upstream/origin, drvPath if present.
- `cix inspect <unit-or-service>` (runtime): same target selection helper as cix exec
  (exact unit / unique service among running cix-* units); show state, MainPID, exit
  cause (last), the effective generated unit properties, port/listener bindings, and the
  host paths of the role directories.
- Output: JSON by default (machine-readable, stable field names); `--human`/`-H` for a
  compact human table. Do not invent a second schema language — field names mirror the
  manifest/entry vocabulary.
- Ambiguity between worlds (a name that is both a tag and a running service): error
  listing both interpretations with the exact commands to disambiguate.

## cix ls -l systems column

Per D35 (e): `cix ls -l` gains a systems column listing the per-system output slots of
each tag (e.g. `x86_64-linux,aarch64-linux`). Keep column layout readable; update any
golden/tour output this touches via the harness (never hand-edit generated pages).

## Ledger

Flip docs/docker.md rows `image inspect` and `container inspect` from ⏳ designed to ✅
citing D35, and the `docker manifest` row's "still missing" note about the systems column.
Dispositions cite decisions; keep ledger style.

## Verification gate

1. cargo build/test/fmt --check/clippy -D warnings clean; unit tests for both dispatch
   paths, the ambiguity error, and JSON schema stability (golden-style).
2. Live (sudo allowed): inspect a tag (artifact world) and a running nginx unit (runtime
   world); transcripts in the LOG. Stop and clean up.
3. Tour: add an inspect scenario page if deterministic under the normalizers (JSON with
   store hashes normalizes well); otherwise explain in the LOG. Drift green either way.
4. `nix build .#checks.x86_64-linux.vm-dogfood` passes.
5. Commit on track/inspect. No commit = failed task.

## Log

Keep .dev/specs/track-inspect.LOG.md current (append-only, timestamped, transcripts).
