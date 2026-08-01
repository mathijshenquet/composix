# Secrets: build-time credentials and runtime delivery

Status: proposal, 2026-08-01. Decision pending. One invariant, two
halves.

## 1. The problem

The invariant: **a secret must never enter the store, a lock, a memo
key, or the journal** — everything in those places is world-readable
and/or forever.

Two holes today. **Build-time**: fetching private dependencies
(playwright's `--mount=type=secret` npmrc, private git/registries
generally — docs/corpus.md §4.8) has no story: FETCH runs with no
credential channel, and docs/migrate.md must mark these migrations ❌.
**Runtime**: services that need a db password or API key have no
delivery mechanism; the ledger defers to "⏳ `LoadCredential=`, compose
era", which is now.

## 2. Prior work

**BuildKit secret mounts** expose a host file at a tmpfs path during one
`RUN`, excluded from layers and cache keys — the exact shape of the
invariant, proven at scale. **Nix** solves fetch credentials at the
*builder* level: `netrc-file` / `access-tokens` are daemon/user
configuration consulted by fetchers, never part of the derivation — so
the same drv builds for anyone with access, and the credential is
invisible to hashing. **k8s Secrets** are cluster objects delivered as
tmpfs files or env vars; env delivery is the documented anti-pattern
(leaks via `/proc`, inherited by children, dumped in crash reports).
**Compose/swarm secrets**: file delivery at `/run/secrets/<name>`;
compose-file `secrets:` saw zero use in our 18-file corpus — but that
measures the *object syntax*, not the need (the same files carry
passwords in env vars instead, i.e. the anti-pattern won by default).

**systemd credentials** are best-in-class and underused:
`LoadCredential=name:path` (also directories, sockets),
`ImportCredential=`, encrypted variants via `systemd-creds encrypt`
(optionally TPM2-bound, per-unit scoped), delivered in a private tmpfs
at `$CREDENTIALS_DIRECTORY/<name>` — not in env, not inheritable, not
visible to other services, works identically under the user manager.
The nix ecosystem (sops-nix/agenix) layers "encrypted in repo, decrypted
at activation" on similar plumbing.

## 3. Recommendation

**Runtime**: the manifest declares needs by name — `SECRET db-password`
(name only; need is app knowledge). Compose supplies sources:
`secrets: { db-password: { file: "/etc/keys/db" } }` (or an encrypted
blob for `systemd-creds`); generation compiles to
`LoadCredential=db-password:<source>`. The app reads
`$CREDENTIALS_DIRECTORY/db-password`. **File delivery only — env
delivery is refused** (the k8s lesson; migration guidance: an app that
only takes `$DB_PASSWORD` gets a shim spelled explicitly in its EXEC, or
better, `_FILE`-convention support that most images already grew).
Declared-but-unsupplied fails `compose check`; supplying an undeclared
secret is the loud loosening case (D49a polarity). `cix run` v0: no
secret flags; secrets arrive with compose.

**Build-time**: follow Nix, not BuildKit — fetch credentials are
**host-level fetcher configuration** (a cix credentials file:
URL-pattern → token/netrc, in `~/.config/cix/` or
`$CREDENTIALS_DIRECTORY` when cix itself runs as a unit), consulted by
FETCH, never spellable in a Cixfile (Cixfiles are committed; a secret
*name* in a Cixfile is a lie waiting to happen since the fetch's
success depends on ambient config either way). RUN needs no secret
channel: it is networkless by design — the private-registry case IS the
FETCH case. Locks record what was fetched (pin + hash), never how it
authenticated; memo keys are unchanged (D48a: keys on inputs — the
credential is not an input, it is access).

## 4. Open questions

1. `_FILE`-convention shim: do we bless/document the pattern only, or
   give `SECRET` an opt-in `env-file:` sugar that sets
   `DB_PASSWORD_FILE=$CREDENTIALS_DIRECTORY/db-password`? Proposal: the
   sugar — it keeps the refusal of raw env delivery while meeting the
   ecosystem convention.
2. Encrypted-at-rest: recommend `systemd-creds encrypt` in docs from day
   one, or plain files first? (TPM binding is host-specific — interacts
   with backup/migration.)
3. Rotation: changing a credential file does not change any store path,
   so restart-changed will not see it. Is `cix up --restart <svc>` /
   documented `systemctl restart` acceptable v0, or does compose need a
   secrets-fingerprint in the generation identity?
4. Does the FETCH credential config need per-Cixfile scoping (two
   projects, two GitHub tokens), or is host-global + URL patterns
   enough for v0?
