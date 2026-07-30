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

## ⚖ Hard choices (Mathijs)

- **Ownership vs DynamicUser — the real fight.** A host-backed state dir defeats the
  DynamicUser idmap (the ocimport finding; and the systemd-261 saga shows id-mapped
  managed dirs are fragile ground). Menu:
  (a) host-bound dirs force a declared static user for that service (docker-honest:
  compose gains `user:`; hardening loss stated loudly),
  (b) keep DynamicUser + recursive chown on activation (mutating operator data —
  ugly, surprising),
  (c) rely on systemd idmapped binds where the host supports them, loud degraded
  fallback to (a) elsewhere (most machinery, version-sensitive).
  Recommendation: (a) as the honest v0, (c) as evidence-gated future.
- **Watch-dirs shared with host users** (Paperless consume): same dir written by a
  human and read by the service — group-based access (operator puts both in a group)
  or ACLs? Recommendation: document the group pattern, build nothing.

## Scope & gate

cix-compose (schema + resolve + generation) + cix-run bind compilation; scenario
asserting a relocated state dir survives up/rollback and ownership behaves per the
chosen model. Corpus receipts for Immich-shaped pair once track/migrate reaches it.
