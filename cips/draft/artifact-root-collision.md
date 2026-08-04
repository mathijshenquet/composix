# artifact-root-collision — role dirs under the application tree (CIP-light)

Status: **draft, CIP-light** (2026-08-04; from wallos's regeneration).

**Problem.** Wallos wanted its upstream layout `/var/www` with state
dirs beneath the app tree, but the artifact's own mount occupies that
root: a role directory cannot be declared at/below certain paths the
artifact mount claims, so the conversion moved the webroot to `/app`
and noted the deviation. This is the same family as CIP-91's
materialization triggers (role dir below a linked COPY destination
already materializes the chain) — but here the collision is with the
artifact root mount itself, and the failure is a layout compromise
rather than a clear diagnostic.

**Proposal.** Decide the honest rule and enforce it at build time with
a spanned error naming the collision — or, if the mount layering can
absorb it the way STATEDIR-below-COPY now does, absorb it. Either
outcome removes the silent layout pressure; the wallos case is the
acceptance test.

**Effort.** Small-medium — mount-layering analysis first, then either
diagnostic or fix.
