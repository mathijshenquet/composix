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

## ⚖ Hard choices (Mathijs)

- **Stable group identity = the first cix-owned host registry beyond profiles.**
  DynamicUser uids are ephemeral by design, so joint ownership must hang on a GROUP
  that outlives service restarts. Menu:
  (a) cix allocates+records real groups per persistent shared edge (a small
  name→gid registry in cix state; deterministic, survives reboots; but it is new
  mutable host state cix must own),
  (b) `DynamicUser` + ACL rewriting on activation (no registry, but ACL churn on
  every up and files keep dead uids),
  (c) require the operator to pre-create the group and name it in compose
  (zero cix state; friction).
  Recommendation: (a) — it is the same kind of cell as the profile: small, named,
  diffable; document it in the host-state inventory.
- **Quota/limits on the shared dir**: none in v0 (honest note), or is that
  acceptable? Recommendation: none, note it.

## Scope & gate

cix-compose edges + cix-run group/dir compilation; scenario: two services write and
read each other's files through the edge, ownership stable across restart AND
rollback; a non-member service provably cannot write. Mastodon corpus pair is the
end-goal receipt.
