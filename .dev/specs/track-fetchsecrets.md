# track/fetchsecrets — secrets for FETCH (corpus demand, build side)

STATUS: design-position spec. One hard choice pending Mathijs (⚖); depends on D47
FETCH forms. Corpus evidence: playwright's `--mount=type=secret` npmrc; private
registries/repos generally.

## Design position

`FETCH <name> --secret <id> <cmd>` (both FETCH forms). The key insight that keeps
this small: **FETCH output is content-pinned (TOFU narHash in the lock), so the
secret is pure ACCESS — it can never affect what counts as the right answer.**
Consequences, stated as guarantees:

- The secret enters neither the memo key, nor the lock, nor the store, nor the
  generated Nix — a rebuild that memo-hits (or substitutes the pinned output) needs
  NO secret at all. Secrets are only required on a true refetch.
- Delivery into the sandbox: a file at a fixed path (`/run/cix-secrets/<id>`,
  tmpfs, mode 0400), docker's type=secret shape — never an env var (env leaks into
  child processes and error output too easily).

## ⚖ Hard choice (Mathijs)

- **Where does the operator provide the secret?** Menu:
  (a) environment at build invocation: `CIX_SECRET_<ID>` (simple, CI-friendly,
  but env-visible in /proc during the build driver's life),
  (b) file reference: `cix build --secret id=/path/to/file` (explicit, no env
  exposure, slightly more typing),
  (c) systemd-creds integration (encrypted at rest; heaviest, host-coupled).
  Recommendation: (b) as primary, (a) as sugar reading a file path from env, (c)
  never unless evidence.

## Scope & gate

cix-cixfile (grammar + sandbox mount + lock behavior); tests proving: secret absent
from item/lock/generated-nix byte-wise; memo-hit rebuild succeeds with no secret
provided; refetch without secret fails with a clear error naming the id.
