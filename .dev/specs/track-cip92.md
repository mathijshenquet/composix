# track/cip92 — protocol-aware ports + `cix build --file`

Read AGENTS.md first (gate convention; synchronous receipts), then
cips/accepted/0092-port-protocols.md — its Decision section binds. Work in
the herdr worktree on branch `track/cip92` (branched AFTER track/cip91
merged — you share parser files with it). Keep `crates/cix-cixfile/LOG.md`
current (dated track heading).

## 1. CIP-92: protocol-aware PORT

- Grammar: `PORT <name> = [udp:]<port>` — systemd style, single form.
  Bare port keeps the tcp default. The Docker spelling (`443/udp`) is a
  parse error whose diagnostic explicitly suggests `udp:443`. No sctp, no
  reserved grammar.
- Compile through: manifest field, `SocketBindAllow=udp:<port>` (and the
  deny mirror) exactly as the tcp path does today, `cix inspect` output,
  and wherever declared ports surface (compose publish stays tcp-only —
  the CIP defers protocol publish honestly; refuse loudly if someone
  publishes a udp port rather than silently binding tcp).
- Tests: parser (incl. the hint), codegen golden for the bind-allow line,
  inspect projection.

## 2. `cix build --file <name>` (Mathijs-sanctioned 2026-08-04, for
translation twins)

- `cix build --file Cixfile.dissolved <dir>[#member]` builds the named
  Cixfile in the directory; default remains `Cixfile`. The lock is the
  sibling `<name>.lock` (`Cixfile.dissolved.lock`). All other behavior
  (selectors, `-t`, `--namespace`, `--update-lock`, `--cold`) is
  unchanged and must compose with `--file`.
- CIP-90 clap boundary rules apply; errors for a missing named file are
  spanned and name the path.
- Tests: build a fixture directory carrying both `Cixfile` and
  `Cixfile.dissolved`, assert independent locks and independent builds.
- Document the flag where `cix build` is documented (tour if the tour
  shows build invocations; docs/migrate.md's verification section
  mentions twins only if it already discusses them — do not introduce
  new canon prose there).

FENCE: track/netnsrace (netns/scenarios) and track/adapterlive (health)
may still be in flight. Do not touch their modules, corpus Cixfiles,
docs/corpus.md prose, or cips/. Your domain: crates/cix-cixfile,
crates/cix-build, crates/cix CLI surface, tests, tour regen, your LOG.

## Gate

Standard agent tier (fmt, examples fmt, warning-denied clippy, full
workspace tests, tour regen+drift). No VM scenario is structurally
affected; if codegen goldens shift, run the focused scenario that
consumes them. Bounded as always. Receipts are synchronous exit statuses.
