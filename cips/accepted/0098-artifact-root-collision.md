# artifact-root-collision — role dirs under the application tree (CIP-light, v2)

Status: **CIP-98, adopted 2026-08-04** (CIP-light; v2 akkoord — role
dirs are declarable anywhere, docker-volume-style nesting).

**Problem.** Wallos wanted upstream's `/var/www` layout with state dirs
beneath it, but declaring role dirs under the artifact-projected tree
failed, so the conversion moved the webroot to `/app` — a silent layout
compromise.

**Analysis (v2).** Nothing speaks against it. Docker mounts volumes
inside image paths routinely; the mechanism is plain mount ordering:
project the read-only artifact path first, mount the writable role
directory inside it second. CIP-91 already materializes the store-side
ancestor chain for exactly this shape, and the CONFIGDIR fix proved the
mirror machinery generalizes. The wallos failure is suspected to be a
validator refusal (like CONFIGDIR's was), not a mount impossibility —
implementation verifies.

**Proposal.** Role dirs are declarable anywhere, including beneath
artifact-projected paths; unit generation orders the RO projection
before the nested RW role mount. Wallos's `/var/www` layout is the
acceptance test (its GAPS bullet flips on landing).

**Effort.** Small-medium: ordering + validator lift + regression.

## Decision

Adopted 2026-08-04 at v2: role dirs anywhere including beneath
artifact-projected paths; RO projection mounts before nested RW role
mounts; validator lifted; wallos /var/www is the acceptance test.
