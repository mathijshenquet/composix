# track/pnpm-frozenstore — the surgical pnpm route: validate, bump targets, build the hints

Charter: `cips/draft/pnpm-wall.md` — read §3 leg 2 (the frozenStore
route, with the upstream receipts) and §4 (the two DECIDED items this
track executes). Direction is Mathijs's, 2026-08-06: make this
problematiek visible to the user and hint the solution; for the
specific targets bump pnpm and record it. SURGICAL is the watchword:
use pnpm's own mechanism end-to-end; do not normalize, strip, or
regenerate any store bytes yourself.

Do, in order:

1. **Validation spike (gates everything else).** Verdaccio-shaped,
   with a pnpm ≥11.7 override (corepack): `pnpm fetch` → seal the
   WHOLE store as fetched (`files/` + `index.db`, TOFU instance-pin)
   → `pnpm install --offline --frozen-lockfile` with
   `frozen-store=true` against that store made READ-ONLY, on Node
   ≥22.15. Run the install twice, independently; receipts must show
   network silence and identical results. If this fails, STOP the
   mechanism legs and record exactly which upstream guarantee broke —
   that finding is the track's product then.
2. **Corpus bumps, recorded.** verdaccio (11.1.2→≥11.7) and directus
   (10.27.0→≥11.7): bump pnpm in the translations, and record the
   deviation explicitly in each GAPS.md ("pnpm upgraded past
   upstream's packageManager pin for frozenStore read-only-store
   support; pre-11.7 pnpm structurally cannot install from a pinned
   store"). Then drive each case as far as it goes under the full
   route — verdaccio to green or a precisely-named wall; directus at
   least through fetch+store-seal (its absent-offline-content wall
   may still stand — honest recording, not forcing). Re-verify dozzle
   (already pnpm 11.17.0) under the same route.
3. **Diagnostics + hints (product change, scoped).** Two problem-class
   hints at cix's existing FETCH/build failure reporting seams:
   - TLS-trust masquerade: FETCH timeout whose trace tail shows
     repeated certificate-probe failures (the ENOENT-on-hashed-certs /
     TLS-handshake pattern) → name the class and hint importing
     `${pkgs.cacert}`.
   - pnpm offline/store walls: `ERR_PNPM_NO_OFFLINE_TARBALL` (and the
     frozen-store error family) in a step's output → name the class
     and hint the frozenStore route incl. its version gates
     (pnpm ≥11.7, Node ≥22.15).
   Hints cite stable doc anchors (D73), never CIP/D numbers. Keep the
   detection conservative — no false-positive hints on unrelated
   timeouts; leave exact-match evidence in tests.
4. **If the double-fetch probe's grading blocks the pin** (index.db
   divergence being treated as refusal rather than recorded
   instance-volatility): a CONFINED reclassification at the probe
   seam is in scope — record it prominently in the LOG and the draft
   (it is the one semantic amendment this route needs). If the change
   turns out NOT confined to the probe seam, stop and report instead.
5. **Ledger discipline**: regrade GAPS/receipt/docs/corpus.md rows you
   actually re-verified; regenerate the corpus browser; migrate.md
   gains the cacert teaching line if it does not have one yet.

Gates (Rust changes expected in leg 3/4): fmt, examples fmt,
warning-denied clippy, `cargo test --workspace`, tour regen+drift if
touched, plus the focused VM scenarios your changes select via
`devenv shell -- nix run .#progressive-vm-check`. Corpus suite for
touched cases. Synchronous value-checked receipts only.

Discipline: branch `track/pnpm-frozenstore`, LOG `crates/cix/LOG.md`
(append, timestamped, FRICTION section). Walls are valid outcomes.
Clean committed branch; do not merge.
