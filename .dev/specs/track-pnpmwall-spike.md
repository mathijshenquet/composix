# track/pnpmwall-spike — the pnpm wall: diagnose the hang class, spike the two-phase store

Charter: `cips/draft/pnpm-wall.md` — read it first (five exhibits,
four-leg recommendation). This track executes the evidence legs (1+2);
it decides NO language changes on its own authority — mechanism specs
go into the draft for Mathijs's adoption call.

Do, in order:

1. **Dozzle hang diagnosis** (leg 1): rerun the dozzle
   `pnpm fetch --ignore-scripts` under a time bound WITH
   `${pkgs.cacert}` imported (the homer fix), pnpm verbose/network
   logging, and strace/`ss` capture. Classify: cacert masquerade /
   IPv6-AAAA-before-v4 fallback / other. Evidence in LOG; conclusion
   written into the draft's exhibit 2.
2. **Two-phase store spike** (leg 2): establish whether pnpm
   regenerates its store index/metadata offline from a bare CAS
   (`…/files/` only) — run `pnpm fetch`, strip derived state
   (SQLite index/metadata), run `pnpm install --offline`, and check
   determinism across two independent runs. Test at the pnpm versions
   verdaccio and dozzle pin. Compare npm's cacache briefly (does the
   same derived/CAS split hold?).
3. **Payoff attempt**: if the spike holds, translate verdaccio to the
   two-phase idiom using EXISTING language features (FETCH runs
   `pnpm fetch`, pinned artifact is the store, install step runs
   `--offline`) and drive it to green — or to a precisely-named wall
   stating exactly which mechanism is missing (that spec text is the
   track's product then). Verdaccio is the consumed-volatility hard
   core; a green here cracks the wall.
4. **Directus** (exhibit 4): confirm the upstream-incoherence
   diagnosis independently; check whether a nearby coherent revision
   exists (this gates CIP-107's 14 narHash-lock regenerations).
5. **Write findings into `cips/draft/pnpm-wall.md`** — evidence into
   the exhibit sections, answers into ch. 4's open questions; status
   stays draft (adoption is Mathijs's). Regrade only the corpus
   GAPS.md rows you actually re-verified.

Walls are valid outcomes; record honestly and continue with the next
leg — do not stall the track on one leg's wall.

Discipline: branch `track/pnpmwall-spike`, LOG `crates/cix/LOG.md`
(append, timestamped, with a FRICTION section — record what mistaught
you). Receipts: synchronous value-checked exit codes only; every claim
leaves an exact repro command. Gates: fmt + corpus suite for touched
cases; `cargo test --workspace` only if Rust changes (expected: none).
Clean committed branch; do not merge.
