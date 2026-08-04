# artifact-root-collision — role dirs under the application tree (CIP-light, v2)

Status: **draft, CIP-light, v2** (2026-08-04; v2 after Mathijs: "role
dirs onder normale dirs vind ik gewoon prima, dat doet docker toch ook
met volumes? Spreekt hier iets tegen?" — answer: nothing fundamental).

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
