# track/spike-runtrace — D38 spike: traced RUN across ecosystems (rust, go, pnpm, uv)

Read AGENTS.md first. Authoritative design: docs/design.md **D38** (the RUN hypothesis).
This is a SPIKE: prototype-quality code is fine, measurements must be real. Nothing lands in
crates/ or the Cixfile language. Everything lives under `.dev/spikes/run-trace/`.

## The hypothesis under test (Mathijs's golden path)

A user writes plain `RUN <ecosystem tool>` and composix hardcodes NO ecosystem knowledge.
Therefore the spike harness MUST be ecosystem-agnostic: one generic mechanism (sandbox +
read tracing + memo check). All cargo/go/pnpm/uv specifics may appear ONLY inside the
per-example project files (their command lines and prefetch scripts). If you find yourself
needing ecosystem logic in the harness, STOP and record that as a (negative) finding — that
is exactly what the spike exists to discover.

## Harness (prototype)

`.dev/spikes/run-trace/harness/` — a small tool/script that:

1. Runs a given command in a sandbox: /nix/store read-only visible, NO network, a writable
   workdir, an explicitly provided environment (PATH of offered tool packages, plus an
   offered "deps dir" where relevant). Reuse whatever is cheapest: unshare/systemd-run;
   root allowed (sudo works in this environment).
2. Traces file reads (open/exec/mmap/stat-that-matters) and reduces them to the set of
   TOP-LEVEL store paths read (`/nix/store/<hash>-<name>`). Tracing tech is your call —
   strace/fanotify/eBPF/FUSE — record the choice and its blind spots in the LOG. Reads
   outside /nix/store (workdir, /proc, deps dir) are excluded from the closure but note
   what shows up there.
3. Memo semantics: store `hash(command) + traced store-path set + output hash`; on re-run,
   hit iff every traced path is offered unchanged; else miss → run + re-trace.
4. Output goes to a directory; hash it (nar-style via `nix store add` or equivalent).

## Matrix (four tiny example projects, in `.dev/spikes/run-trace/examples/`)

- **rust via cargo-chef**: the flagship. Small bin crate with 2–3 real deps including one
  proc-macro (e.g. serde+derive). TWO RUN steps exactly as a Dockerfile would:
  `RUN cargo chef cook --recipe-path recipe.json` then `RUN cargo build --release`,
  second offered the first's output. This tests the primitive's composability.
- **go**: small module with 1–2 deps, `RUN go build` with a pre-vendored dir (vendor/
  committed or prefetched), GOFLAGS=-mod=vendor, GOPROXY=off.
- **pnpm**: small package with a build script (esbuild-class), offline pnpm store
  prefetched, `RUN pnpm install --offline && pnpm build`.
- **uv**: small python package with 1–2 deps, wheels prefetched into a local cache/dir,
  `RUN uv sync --offline` (+ a trivial invocation proving the env works).

Prefetching MAY use the network (it happens outside the sandboxed RUN and stands in for
what the product would derive from locks as FODs — say so in the report). Toolchains come
from nixpkgs on the offered PATH.

## Measurements (per ecosystem — this IS the deliverable)

a. Offline sandboxed build succeeds: yes/no (+ what it took).
b. Traced store-closure stability: run twice with identical offers — is the closure
   set-identical? List any noise paths and their cause.
c. Output stability: byte-identical outputs across the two runs? If not, what embeds
   nondeterminism (timestamps, ordering) — per ecosystem.
d. Miss semantics: change one input (bump a dep / edit source), verify the memo misses,
   re-runs, and the trace updates.
e. Harness purity: did the harness stay ecosystem-agnostic? Every place it could not is a
   finding.
f. Tracing overhead: rough wall-clock with vs without tracing.

## Deliverable

`.dev/spikes/run-trace/REPORT.md` — the matrix table (a–f × 4 ecosystems), the verdict
against D38's promotion gate ("a real cargo chef cook traces to a stable closure across
runs"), surprises, and the sharpest 3 open problems for productization. Keep
`.dev/specs/track-spike-runtrace.LOG.md` current throughout (append-only, timestamped,
transcripts). Do NOT edit docs/design.md — the orchestrator folds the evidence into D38.
Commit everything on track/spike-runtrace. No commit = failed task.
