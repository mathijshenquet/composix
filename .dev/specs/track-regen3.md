# track/regen3 — wave-3 regenerated corpus cases (luna cold staging → assembly)

DO NOT LAUNCH until track/envgrammar and track/cip102 are merged: both
change migrate.md and corpus Cixfile canon; regenerating against the
older prompt would go stale on arrival.

Process mirrors regen1/regen2 exactly (read .dev/specs/track-regen2.md
and docs/corpus.md §"How this corpus is maintained") unless stated
below. Branch `track/regen3`, herdr worktree; LOG:
`corpus/migrate/LOG.md`. Four cold staging dirs (fresh luna agents,
each gets ONLY docs/migrate.md + the case Dockerfile + context):
`/home/mathijs/regen-stage/{caddy,parse-server,directus,watchtower}`.
Header: `Generated: migrate.md@<staging commit> · gpt-5.6-luna ·
<date>`. Two-layer warm/cold evidence rule applies unchanged.

## Per-case expectations (the point of this wave)

- **caddy** — regenerate with CONFIGDIR path freedom and the landed
  localhost skeleton: the minimal `/etc/hosts` `FILE` workaround and
  the CONFIGDIR deviation must be GONE from the fresh translation;
  verify the four-socket contract (incl. `udp:443`) still probes. The
  old GAPS links a now-removed open-questions anchor — the fresh GAPS
  simply drops it.
- **parse-server** — CONFIGDIR deviation gone. The cold node_modules
  read-set divergence may well persist: record honestly per the
  two-layer rule, do not re-pin to hide it.
- **directus** — the headline: CIP-95 imported glibc + the traced
  ENOTDIR fix should carry the build past both former walls. If an
  item builds, probe it and grade the remaining offline-deploy
  boundary honestly (its row is ⏳ pending exactly this). If a new
  wall appears, exact error + gap bullet, no forcing.
- **verdaccio** (added, Mathijs 2026-08-05) — retry now that
  buildfixes traced ENOTDIR-as-absent-read (directus's blocker; same
  error class). Cold regen; if `Not a directory` persists, the
  diagnosis now names the missing path — record it exactly, that
  precision is the deliverable even on failure.
- **filestash** (added; NOT cold — investigation) — the static
  C-library wall meets the `pkgsStatic` universe: it is reachable as
  ordinary attribute paths (`${pkgs.pkgsStatic.<lib>}`), so iterate
  the missing-lib loop: build → cgo link error names `-l<name>` →
  match `pkgs.pkgsStatic.<name>` → rebuild. Record each iteration in
  the LOG (this measures whether the loop converges — feed for a
  future lint/tooling idea). Honest wall if the set stays incoherent.
- **dozzle + watchtower** (decision, Mathijs 2026-08-05: "schrijf ze,
  testen hoeft niet; de socket is interessant als demo") — rewrite
  both as **docker-socket-bridge** conversions: the Cixfile declares
  the host's `/var/run/docker.sock` via the documented static-identity
  `host:` materialization and the app runs against a real dockerd
  where one exists. UNTESTED by design: the gate VM has no dockerd, so
  the receipt records evidence class "desk (socket bridge, unprobed —
  no dockerd in the gate)" and check.sh probes only what needs no
  daemon. Keep the thesis note in each GAPS: journald/`cix logs` and
  nix pins/`cix` updates are the native composix answers; the bridge
  demonstrates coexistence, it does not test migration. Watchtower's
  stale duplicate-COPY workaround disappears in the rewrite. If the
  socket `host:` materialization itself hits a language wall, that
  finding is a gap bullet `→ language`, not something to force.

## Ledger + close

Re-grade the affected docs/corpus.md rows using the NEW ribbon
vocabulary (landed on main before this track launches): green = works,
remaining deviations deliberate/refused/otherwise-arranged; 🔶 = open
gaps, always qualified — 🔶🔄 next regen improves it, 🔶⌛ fix adopted
but unimplemented. Update affected docs/docker.md rows in the same
track; GAPS `Status: current` (or honest stale-with-reason); rerun
`check.sh` receipts synchronously per case (dozzle/watchtower: the
declared daemonless subset only); regenerate the corpus browser.
After this wave lands, the corpus EXPANSION wave from
docker/CANDIDATES.md is next in the queue (Mathijs wants this
visibly happening) — do not start it in this track. Track gates per AGENTS.md (fmt / examples fmt /
clippy / workspace tests / tour drift / progressive-vm-check for what
the diff selects). Leave the branch clean; do not merge to main.
