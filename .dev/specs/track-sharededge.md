# track/sharededge — shared-rw directory between services (corpus demand #3)

STATUS: design-position spec. Hard choices pending Mathijs (⚖); do not launch until
D-numbered. Corpus evidence: Mastodon (`public/system` written by web AND sidekiq),
Penpot (assets volume rw-shared frontend+backend). Today's edges are
producer→consumers sockets/paths; there is no shared writable surface.

## Design position

A new edge kind: `"kind": "shared-dir"` — a directory jointly writable by its member
services. Two flavors:

- **runtime** (`/run`-backed, tmpfs, gone on down): trivial — per-edge group +
  setgid dir, the proven dstyle mechanism extended to rw for all members.
- **persistent** (state-backed): the real demand (uploads!). Backing dir under the
  composite's state, owned by a per-edge group; members get the group as
  SupplementaryGroups; dir is setgid + `g+rw`, umask coordinated.

## Resolved (D48d — Mathijs's round)

- **Same problem class as track-hostbinds** (Mathijs: "dit lijkt heel erg op 2"):
  durable data ownership = declared identities. The stable group here and the static
  user there come from ONE cix-managed identity registry (name→uid/gid, profile-like
  cell; cix-allocated default, operator-precreated expressible). Implement the
  registry once — in whichever track runs first — and consume it in the other.
- Quota/limits on the shared dir: none in v0, honestly noted.
- Registry provenance (Mathijs, 07-30): **cix allocates.** Noted-not-built escape
  hatch for when it ever chafes: a *donation* flow — move existing user data into
  the cix-managed location and symlink back at the old path (one truth afterwards;
  strictly better than seeding/copying). Do not build it now.
- Co-location: shared dirs are fs-tier — members are co-location-constrained per
  compose-tree §3b; a future multi-host split may never cut through this edge.

## Scope & gate

cix-compose edges + cix-run group/dir compilation; scenario: two services write and
read each other's files through the edge, ownership stable across restart AND
rollback; a non-member service provably cannot write. Mastodon corpus pair is the
end-goal receipt.
