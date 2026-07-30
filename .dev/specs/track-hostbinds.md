# track/hostbinds — operator host-directory binds (corpus demand #2)

STATUS: design-position spec. Hard choices pending Mathijs (⚖); do not launch until
D-numbered. Corpus evidence: Immich (`${UPLOAD_LOCATION}:/data`), Paperless
(consume watch-dir), Mastodon (bind-mount-only deployment) — env-interpolated,
operator-chosen host paths backing service data.

## Design position

The primitive is NOT "arbitrary mounts" (D11 deliberately narrowed that) but
**relocating a declared role dir**: compose per-service
`"dirs": { "state": "/srv/immich-uploads" }` — the manifest still declares the role
(app-semantic, D20), the operator chooses its host backing (operator-semantic).
Compiled as a bind from the host path onto the managed location, so the app sees the
same declared path. Paths must be absolute, outside /nix, existing (up-time check,
loud error).

## Resolved (D48d — Mathijs's round)

- **A user that owns the data is the clean solution**: host-bound dirs require a
  declared static user for that service (hardening delta stated loudly). In `--user`
  mode the problem dissolves — everything runs as the invoking user and binds are
  their own files; note this explicitly in docs.
- **Identity provenance is the SHARED decision with track-sharededge**: static
  users (here) and stable groups (shared edges) come from one small cix-managed
  identity registry (name→uid/gid, profile-like cell); operator-precreated
  identities remain expressible. Implement the registry once, in whichever of the
  two tracks runs first; the other consumes it.
- Watch-dirs shared with host humans (Paperless consume): document the group
  pattern via the same registry; build nothing extra.

## Scope & gate

cix-compose (schema + resolve + generation) + cix-run bind compilation; scenario
asserting a relocated state dir survives up/rollback and ownership behaves per the
chosen model. Corpus receipts for Immich-shaped pair once track/migrate reaches it.
