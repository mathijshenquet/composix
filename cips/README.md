# CIPs — Composix Improvement Proposals

Process adopted 2026-08-01 (Mathijs), successor to raw design.md D-numbers
for new decisions.

## Process

- **Drafts** live in `cips/draft/<name>.md` — name only, no number.
  Anyone (usually the orchestrator) writes one when a design question
  surfaces; the author posts Mathijs the full GitHub URL.
- **Adoption** is Mathijs's call. On adoption the file moves to
  `cips/accepted/<NNNN>-<name>.md`; numbering continues the design.md
  D-sequence (D74 was the last D-number, so the first CIP is **CIP-75**,
  filename `0075-<name>.md`) — one unambiguous citation sequence across
  both eras.
- **Amendments**: while composix is v0, landed CIPs may be amended in
  place. Every amendment appends a dated line to the CIP's Changelog
  section. Post-1.0, amendments become new CIPs that supersede.
- **Rejected** drafts stay in `draft/` with `Status: rejected (<why>)` —
  refusals are records (feature-level refusals also land in the
  docs/docker.md ledger). Superseded pre-CIP explorations are filed the
  same way (`Status: superseded`).
- **Retro-adoption & consolidation** (alpha rule, Mathijs 2026-08-02):
  pre-CIP design papers may be renumbered into the CIP sequence, and
  clusters of design.md D-numbers may be cleaned up and consolidated
  into one CIP. The consolidating CIP names the D-numbers it absorbs;
  the design.md entries stay citable and gain a pointer. Neither
  sequence needs to stay gapless — one clear system beats a contiguous
  range. First instances: CIP-85 (compose tree, D40–D46) and CIP-86
  (netns realization, D49).

## Template

Four chapters (per Mathijs's format), plus one on adoption:

1. **The problem** — from zero context, concise.
2. **Prior work** — may be thicker.
3. **Recommendation**.
4. **Open questions** — each one answerable with a short taste call.
5. **Decision** (added at adoption) — what was decided, including the
   open-question answers, plus the Changelog.

## Relationship to design.md

D1–D74 in docs/design.md remain authoritative citations; nothing is
renumbered. New decisions land as CIPs. An amendment to an old D-number
is a CIP that names it. Per the D73 addendum, user-facing diagnostics
cite stable doc anchors, never D- or CIP-numbers.
