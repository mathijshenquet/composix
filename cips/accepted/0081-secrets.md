# Secrets: build-time credentials and runtime delivery

Status: **CIP-81, adopted 2026-08-01** (Mathijs: "secrets is prima",
after the r2 consent-store turn-over). One invariant, two halves.
Decision = §3 as amended by §4's resolved answers: file-only runtime
delivery via `SECRET name [AS VAR_FILE]` → `LoadCredential=`;
build-time fetch credentials as host-level named tokens with a
direnv-allow-shaped consent store keyed (project, token, URL-prefix) —
locks and Cixfiles never mention tokens; salted-HMAC rotation
fingerprint in composite state, never the store.

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
`ImportCredential=`, delivered in a private tmpfs at
`$CREDENTIALS_DIRECTORY/<name>` — not in env, not inheritable, not
visible to other services, works identically under the user manager.
The encrypted variant: `systemd-creds encrypt` seals a blob with a
host key (`/var/lib/systemd/credential.secret`, root-only) and/or a
TPM2-sealed key; the ciphertext may sit world-readable on disk or in a
repo; `LoadCredentialEncrypted=` has pid 1 decrypt it at service start,
optionally bound to one unit name so a blob stolen for unit A cannot be
fed to unit B. Rotation = re-encrypt the file; the host key never leaves
the machine.

**agenix / sops-nix** (nix ecosystem): secrets live *encrypted in the
repo*, keyed to recipients' ssh/age public keys (agenix: an
`age`-encrypted file per secret, `secrets.nix` maps files → host keys),
and are decrypted at system activation to `/run/agenix/<name>` with
declared owner/mode. The insight to steal: the *committed artifact*
names and routes secrets while the *host key* gates them — exactly our
manifest-declares / host-supplies seam. Their `/run/<tool>/<name>` file
contract is also LoadCredential-shaped, so agenix-managed files slot
directly into our compose `file:` sources with zero integration.

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

## 4. Open questions — resolved in review

1. **The `_FILE` sugar, precisely**: `SECRET db-password AS
   DB_PASSWORD_FILE` — the optional `AS <var>` sets environment variable
   `DB_PASSWORD_FILE=$CREDENTIALS_DIRECTORY/db-password` in the unit.
   The env var carries a *path*, never a value, so the refusal of raw
   env delivery stands while the ecosystem's `_FILE` convention (most
   official images honor it) works unmodified. Without `AS`, the app
   reads `$CREDENTIALS_DIRECTORY` itself.
2. **Encrypted-at-rest**: plain `file:` sources first; document
   `systemd-creds encrypt` + `encrypted:` source as the recommended
   posture (TPM binding is host-specific and interacts with
   backup/migration — operator's informed choice, not a default).
3. **Rotation fingerprint: yes, but salted and out of the store.** A raw
   content hash in the generation identity would put a secret's digest
   in the world-readable store — an offline confirmation/guessing oracle
   for low-entropy secrets. Instead `cix up` keeps an HMAC of each
   secret's content under a random per-composite salt in *composite
   state* (never the store), compares on up, and restarts services whose
   secrets changed. Restart-changed semantics without leaking a
   fingerprint.
4. **FETCH credential scoping — turned over 4× (requested), and the
   lock drops out.** The v1 mechanism (named tokens + whitelist recorded
   in `Cixfile.lock`) fails three of four adversarial turns:

   - *Turn 1 — committed consent is cloned consent.* The lock travels
     with the repo; if it authorizes token use, cloning (or merging a
     PR that edits the machine-written-but-plain-text lock) means a
     malicious repo arrives **pre-consented** on any host holding a
     matching token. Consent must be host-local; a committed file can
     only ever carry *intent*.
   - *Turn 2 — host-local names must not travel.* Token names are
     per-host config (`gh-work` here, `github-main` there): a lock
     pinning a name breaks on the next host, or silently matches a
     *different* token that shares the name — the wrong secret, used
     without error.
   - *Turn 3 — the lock has no remaining job.* Its job is pinning
     content (rev+narHash, D69), and `--cold` never fetches (D69e), so
     credentialed fetching happens only at update-lock/first-fetch time
     — exactly where a prompt can live. Intent is already public in the
     Cixfile's FETCH URL. Drop lock involvement entirely: simpler
     artifact, smaller attack surface.
   - *Turn 4 — the residual risk is broad URL patterns.* A token scoped
     `*.corp.internal` plus an attacker-edited FETCH URL inside that
     pattern sends the credential to attacker-reachable infrastructure.
     Mitigations: the consent prompt shows the **concrete URL**, not
     just the token name; consent is stored per (project, token,
     URL-prefix) so a new prefix re-prompts; docs push narrow patterns.

   **Resulting design (direnv-allow prior art):** host credential map of
   named tokens (name → URL pattern + credential) + a host-side consent
   store keyed (project path, token name, URL prefix). First use
   prompts "allow FETCH of <url> using <name>? y/N" (`--allow-secret`
   for CI); revocation is a cix command editing host state; a removed
   token fails the fetch loudly (never a silent anonymous retry). Locks
   and Cixfiles never mention tokens. Two tokens matching one URL: the
   prompt disambiguates and the *consent store* (host-side) remembers
   the choice. Hygiene notes that ride along: the D69 probe and
   consumed-set recording must never persist credential paths
   (pinkeys-class volatile-fact discipline), and auth headers never
   reach logs.

## Changelog

- 2026-08-01: drafted; r1 after review — agenix/systemd-creds prior
  work deepened, `AS <var>` sugar spelled, salted-HMAC rotation
  fingerprint, named-token lock whitelist. r2 same day — the requested
  4× turn-over replaced the lock whitelist with host-side consent
  (direnv-allow shaped); locks no longer participate.
- 2026-08-02: implemented file-only LoadCredential runtime delivery and host-consented FETCH credentials.
