# `cix run` is degenerate unary compose

Status: **CIP-77, adopted 2026-08-01** (Mathijs: "helemaal prima en de
juiste vorm"), with the translation-quality guard in §5. Extracted from
Mathijs's review of the binds/timers/devices drafts, where the principle
recurred three times.

## 1. The problem

`cix run` (one artifact, one transient unit) and compose (a composite of
services) each grow operator-facing knobs: binds, schedules, secrets
sources, device grants, env. Without a rule, the two surfaces drift —
run grows flags compose can't spell, compose grows fields run can't
take, and every new feature asks "and what about cix run?" as a fresh
design question.

## 2. Prior work

**Docker** is the cautionary tale: `docker run` flags and compose fields
are two vocabularies for one model, maintained separately for a decade —
flags without compose equivalents, compose fields without flag
equivalents, and a permanent documentation tax explaining the mapping.
**kubectl run** is the other pole: it once grew generators for
deployments/jobs/cronjobs — an ad-hoc surface shadowing the declarative
one — and upstream amputated it back to "one pod, nothing else" because
the shadow surface rotted faster than the real one. Lesson from both
sides: an imperative convenience surface survives only as a *projection*
of the declarative model, never as a sibling.

## 3. Recommendation

Adopt as a design invariant: **`cix run` is compose with one anonymous
member.** Every operator knob is defined once, as a compose field; run
exposes a flag *iff* it is the mechanical spelling of that field
(`--dir state=host:/path`, `--schedule "..."`, future secrets/device
overrides), with the same names, same semantics, same check-loudness
(loosening warnings included). No run-only concepts, no compose-only
concepts that a single service could meaningfully use. New-feature
design then answers the run question by construction: if the compose
field exists, the flag is free; if a proposed flag has no compose field,
that is the smell.

Implementation direction (not mandated by this CIP): run may literally
construct an in-memory unary composite and share the compose
generation path — the invariant is about surface, but convergent
plumbing is the natural way to keep it true.

## 4. Open questions

1. Grandfathering: existing `cix run` flags (`--detach`, `-p name=addr`,
   `--user`) — audit against the invariant now or as the compose fields
   land? (`-p` already mirrors listener binding; `--detach`/`--user` are
   run-mode, not model, and exempt.)
2. Does the invariant eventually collapse `cix run` into
   `cix up --anonymous` internally? (No user-facing consequence; left to
   implementation.)

## 5. Decision

Adopted, with a **translation-quality guard** (adoption condition): the
flag→field translation must stay *mechanical* — a flag is a flat
spelling of one JSON field (`--dir state=host:/path`,
`--schedule "..."`) with no flag-only grammar, no nesting DSL, no
defaults the field doesn't have. The escape hatch is pre-agreed: if a
field's translation turns contrived, that field gets **no flag** — the
k8s route, where the user hands `cix run` an anonymous compose JSON
(`cix run --compose <file|->`) instead. Contrivedness of the
translation, not completeness of the flag set, is the review criterion.
Grandfathering audit (§4.1) lands with the first compose field that
touches run.

## Changelog

- 2026-08-01: drafted from review; adopted same day with the
  translation-quality guard.
