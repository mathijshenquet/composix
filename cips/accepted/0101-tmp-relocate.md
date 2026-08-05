# CIP-101: tmp-relocate — probe/audit scratch: cleanup-first, /var/tmp for big trees (CIP-light)

Status: **accepted** (2026-08-05; CIP-light. v2 was the design round
Mathijs asked for: what does prior work do, what does systemd say, and
cleanup-first).

**Problem (unchanged).** `--update-lock` probes and `--cold` audits
unpack full work trees under /tmp (`cix-fetch-probe-*`,
`cix-build-cold-*`) and leave them behind on failure paths. Node-shaped
trees exhausted this host's tmpfs inode cap and wedged every tool.

**Prior work (researched).** systemd's file-hierarchy guidance: /tmp is
for SMALL short-lived files (often tmpfs; systemd-tmpfiles ages it,
default 10 days) and explicitly says large or longer-lived temporary
data belongs in **/var/tmp** (disk-backed, aged ~30 days) — or a
program-owned cache dir. Nix itself builds under $TMPDIR (default
/tmp) but REMOVES the build dir on completion, keeping it only with
explicit `--keep-failed`. So prior work's answer is twofold: clean up
always, and put big trees on disk-backed storage.

**Proposal (v2).**
1. **Cleanup is the primary fix**: probe/audit scratch is removed on
   every exit path (success, failure, signal), nix-style; a
   `--keep-scratch` flag opts into retention for debugging.
2. **Destination follows systemd guidance**: big-tree scratch goes to
   `/var/tmp/cix-…` (disk-backed, tmpfiles-aged as backstop) or
   `$XDG_CACHE_HOME/cix/tmp` — pick one in implementation; both honor
   `$TMPDIR` override. Small temp files may stay on /tmp.
3. **Orphan sweep**: on startup, remove own scratch older than a day
   (belt-and-braces under the tmpfiles aging).

**Effort.** Small.

## Decision

Adopted as proposed in v2 (Mathijs, 2026-08-05: "prima"): cleanup on
every exit path is the primary fix, big-tree scratch moves to
disk-backed storage per the systemd guidance (implementation picks
`/var/tmp/cix-…` or `$XDG_CACHE_HOME/cix/tmp`), startup orphan sweep
as backstop.

Changelog:
- 2026-08-05 — adopted as CIP-101.
- 2026-08-05 — implementation chooses `/var/tmp/cix-*`: it follows the
  systemd large-temporary-data guidance without making build scratch compete
  with durable cache entries; an explicit `TMPDIR` still wins for callers and
  tests.
