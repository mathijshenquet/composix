# tmp-relocate — probes and cold audits stop littering /tmp (CIP-light)

Status: **draft, CIP-light** (2026-08-04; from the tmpfs-inode incident).

**Problem.** `cix build --update-lock` probes and `--cold` audits
unpack full work trees under /tmp (`cix-fetch-probe-*`,
`cix-build-cold-*`) and leave them on exit. Node-shaped trees carry
hundreds of thousands of tiny files; on this host they exhausted the
tmpfs's ~1.05M inode cap and wedged every tool on the machine. The
tmpfs limits are host-admin-owned, so the fix must be product-side.

**Proposal.** Probe/audit scratch moves to a disk-backed product dir
(`~/.cache/cix/tmp/…`), always removed on exit (success and failure
paths), with a startup sweep of orphans older than a day.

**Effort.** Small.
