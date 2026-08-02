# track/secrets — CIP-81: SECRET/LoadCredential + fetch-token consent, plus run --compose

Read AGENTS.md first (synchronous-receipts convention). Authoritative:
cips/accepted/0081-secrets.md (§3 as amended by §4's resolved answers —
read all four FETCH turn-overs; the design IS those refusals). Work in
`/home/mathijs/worktrees/composix/track-secrets` (herdr worktree) on
branch `track/secrets`. Keep `crates/cix-run/LOG.md` current.
PARALLEL FENCE: track/readset runs concurrently and owns the builder
engine, memo/keying, and trace code in crates/cix-cixfile — your FETCH
credential consultation may touch the fetch execution path only, keep
it additive there; never touch keying/memo (the CIP already demands
credentials stay out of keys, probes, and logs).

1. **Runtime half**:
   - Cixfile `SECRET <name> [AS <VAR>]` on SERVICE/APP → manifest
     field; `AS` sets env `<VAR>=$CREDENTIALS_DIRECTORY/<name>` (a
     PATH, never a value — raw env delivery stays refused with a
     teaching error).
   - Compose `secrets: { <name>: { file: "/path" } }` and
     `{ encrypted: "/path" }` → `LoadCredential=` /
     `LoadCredentialEncrypted=` on exactly the declaring units.
   - Polarities: declared-but-unsupplied fails `compose check`;
     supplying an undeclared secret is the LOUD loosening case (D49a).
   - Rotation: `cix up` keeps a salted HMAC per secret in composite
     STATE (random per-composite salt; never the store, never a raw
     hash) and restarts services whose secrets changed. Prove
     restart-changed in the VM.
   - `cix run` v0: no secret flags (per the CIP) — but see item 3.
2. **Build-time half**: host credential map (named tokens: name → URL
   pattern + credential; `~/.config/cix/credentials` or
   `$CREDENTIALS_DIRECTORY` when cix runs as a unit) consulted by
   FETCH; Cixfiles and locks NEVER mention tokens. Host-side consent
   store keyed (project path, token name, URL prefix): first use
   prompts with the CONCRETE URL ("allow FETCH of <url> using <name>?
   y/N"), `--allow-secret` for CI, revocation command edits host
   state, removed token fails the fetch loudly (no silent anonymous
   retry), two matching tokens disambiguate via prompt + consent
   store. Hygiene: credential paths never persist in probe/consumed
   records; auth material never reaches logs — add tests asserting
   both.
3. **`cix run --compose <file|->`** (CIP-77's pre-agreed escape hatch,
   still unbuilt): accept an anonymous compose JSON on the run
   surface. This also becomes the documented route for secrets with
   `cix run`, and `--dir PATH=host-idmap:` retires onto it (fold the
   fused spelling out with a migration-grade pointer; the CIP-77
   grandfathering audit flagged it as the one guard violation).
4. **Docs/ledgers**: docker.md secrets rows + BuildKit secret-mount
   row + `docker pass`/credential-store rows honestly updated;
   corpus.md demand #10 re-graded (receipt only if you run one);
   migrate.md secret guidance rewritten (the `_FILE` shim story);
   docs/cixfile.md SECRET; compose docs for `secrets:`.
5. **Tests**: parser/fmt round-trips; schema accept/reject incl. both
   polarities; unit-gen fixtures (file, encrypted, AS-var ×
   system/user); consent-store unit tests (prefix keying, re-prompt on
   new prefix, revocation, loud failure); new
   `nix/scenarios/secrets.nix`: member reads
   `$CREDENTIALS_DIRECTORY/<name>` delivered from a root-only file,
   `_FILE` shim works, secret rotation restarts exactly the consuming
   member, undeclared-supplied is loud, `run --compose` starts a
   member with a secret.

Gate: fmt / `cargo run -- fmt --check examples` / warning-denied
clippy / workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. All receipts synchronous.
Commit on this branch when green.
