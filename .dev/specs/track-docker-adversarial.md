# Track: docker-adversarial — make docs/docker.md honest, thick, and sourced

`docs/docker.md` (the Docker ledger) currently reads too kindly toward composix: dispositions
without receipts, no account of what composix genuinely lacks. Your stance in this track is
ADVERSARIAL: argue Docker's side. Every `🔁 adapted` claim must earn it or gain a residual;
every `❌ rejected` must state what is genuinely given up. You are the red team; comfort is
failure. Read `docs/design.md` first so your criticism is informed, not strawmanned.

## Ground rules

- Territory: `docs/docker.md` ONLY (plus your commits). Do not touch design.md, README, code.
- Do not *change* any disposition on your own authority — where you believe a disposition is
  wrong or cope, add/extend a `❓` marker with your argument in one tight sentence; Mathijs
  decides.
- COMMIT AS YOU GO; done gate includes clean `git status --short`.

## Deliverables

1. **Receipts.** Every table row's docker-side term links to its authoritative page under
   https://docs.docker.com/… (reference pages for CLI commands and Dockerfile instructions,
   concept pages otherwise). Spot-check ~15 links with `curl -sI` (2xx/3xx) and prefer linking
   patterns you verified; note the check in a commit message.
2. **Residuals.** After each section's table, a short `**Residuals**` block: what Docker
   delivers here that composix today does NOT — concrete, falsifiable statements (e.g. cold
   transfer sizes, ecosystem scale, platform support), not vibes. Where a residual is planned
   away by a design decision, say which and whether the plan actually covers it.
3. **The honest gaps** — a new top-level section near the top, adversarially written. Cover at
   least, in your own judgment and words: inability to run existing OCI/docker images at all
   (no interop layer); Linux+systemd-only (no macOS/Windows dev story vs Docker Desktop);
   ecosystem scale (Hub's millions of images and pull-through tooling vs cixpkgs' two
   examples); maturity and audit surface of security defaults; rootless completeness;
   networking (no per-app netns/DNS today); operational tooling (monitoring/CI/testcontainers
   integrations); the migration cliff for a docker-composed shop. Add gaps you find yourself —
   the list above is a floor, not a ceiling.
4. **Thoroughness pass.** Any docker concept still missing from the ledger (check the CLI
   reference index while you're in the docs): add it with a `❓` disposition rather than
   inventing one.

## Done gate

Ledger reads like it was written by a skeptic who knows both systems; links verified pattern;
committed; final summary as your last commit message body.
