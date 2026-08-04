# optional-env — declaring an ENV with no default (CIP-96, was CIP-light)

Status: **CIP-96, adopted 2026-08-04** (Mathijs: "docker heeft ook gewoon ENV NAME toch? gewoon doen").

**Problem.** `ENV NAME = value` sets a default and `ENV NAME required`
demands an operator value — but there is no way to declare "this
variable is meaningful, optional, and has no default". Luna's adminer
conversion invented a sentinel (`ENV ADMINER_DESIGN = __cix_unset__`)
plus entrypoint logic to strip it: exactly the workaround shape the
language exists to prevent.

**Proposal.** `ENV NAME optional` — declared in the manifest, absent
from the environment unless the operator supplies it. Grammar mirrors
`required`; runner passes it through only when set.

**Effort.** Small — grammar + manifest field + runner pass-through +
one corpus case cleanup.

## Decision

Adopted 2026-08-04. Spelling: the bare form `ENV NAME` (familiar from
docker-compose's environment passthrough lists) declares an optional
variable with no default; `ENV NAME optional` is not minted. Manifest
records it; the runner passes it through only when the operator
supplies a value. Adminer's `__cix_unset__` sentinel cleanup rides the
implementing track.
