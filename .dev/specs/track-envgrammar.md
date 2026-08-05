# track/envgrammar — implement CIP-100 (ENV NAME=value) + CIP-96 (optional ENV)

Read first: `cips/accepted/0100-env-equals.md`, `cips/accepted/0096-optional-env.md`,
`AGENTS.md` (gates, receipt discipline), and the parser in `crates/cix-cixfile`.

## Scope

One coherent ENV grammar change, both CIPs in one track:

1. **CIP-100**: canon becomes `ENV NAME=value` (no spaces around `=`,
   quotes for values with spaces, exactly like bash/systemd). The old
   spaced form `ENV NAME = value` becomes a parse error whose message
   teaches: "write `ENV NAME=value`". Alpha: no compat, no dual grammar.
2. **CIP-96**: the full coherent set per the CIP-100 proposal block:
   - `ENV PORT=8080` — default value
   - `ENV API_TOKEN required` — mandatory, no default
   - `ENV NAME` — bare: optional, no default (unset stays unset — the
     `__cix_unset__` sentinel class dies)
   - `ENV PORT=8080 required` — parse error (required forbids a default)
3. **Mechanical sweep**: every ENV in corpus/, examples/, docs/tour/,
   docs/cixfile.md, docs/migrate.md moves to the new grammar. Tour is
   regenerated, not hand-edited. migrate.md's teaching prose updates to
   the new canon (keep the edit minimal and in the prompt's voice).
4. **Ledger currency** (AGENTS.md rule): grep `corpus/migrate/{docker,k8s}/*/GAPS.md`
   for CIP-96/optional-env exhibiting cases (adminer) and flip them to
   `Status: stale — regenerate with CIP-96`; same for any case whose
   GAPS cites the ENV grammar. Re-grade affected docs/docker.md and
   docs/corpus.md rows if their prose mentions the sentinel/grammar.
   Regenerate the corpus browser (`cargo test --test corpus -- --ignored
   generate_corpus_browser`).

Out of scope: any manifest-format change beyond what optionality needs
(if the manifest must distinguish optional-unset, keep it minimal and
document in the CIP changelog); build-args (separate draft).

## Discipline

- Branch `track/envgrammar`, this worktree. Log: append timestamped
  entries to `crates/cix-cixfile/LOG.md` (create if absent).
- Gates (all synchronous exit-status receipts, record exact commands in
  the LOG): `cargo fmt --all --check`, `cix fmt --check examples`,
  warning-denied clippy, full workspace tests, tour regen + drift
  check, `devenv shell -- nix run .#progressive-vm-check` for the
  focused scenarios your diff selects.
- A receipt is a synchronous exit status you observed. Never read
  detached output as success.
- If any grammar question is not answered by CIP-96/CIP-100 (e.g.
  quoting edge cases), pick the bash/systemd-conventional answer,
  record it in the LOG, and flag it in your final report — do not
  invent new surface.
- Commit granularly; leave the branch clean (no stray files, `git
  status` empty at the end). Do not merge to main.
