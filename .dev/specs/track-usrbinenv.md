# track/usrbinenv — D58 addendum: /usr/bin/env in the builder sandbox skeleton

Read AGENTS.md first. Authoritative: design.md D58's 2026-07-31 addendum.
Scope: crates/cix-cixfile (sandbox/skeleton setup in the build chain), docs
(cixfile.md builder-environment section, migrate.md if it mentions shebangs),
corpus echo-server re-check. SEQUENCED AFTER track/absdest merges (same crate;
skeleton changes touch chain-key inputs).

1. The fixed builder sandbox gains `/usr/bin/env` as a symlink to the union
   `/bin/env`. Exactly this one entry; no other /usr content, ever (the
   NixOS-two-paths boundary is in the D-text). It dangles when nothing
   imported ships `env` — verify the resulting error on a script exec is
   loud and comprehensible; if it is confusing ENOENT soup, improve the
   diagnostic to mention IMPORT ${pkgs.coreutils}.
2. Chain keys: decide-and-document (in the LOG) whether the skeleton change
   participates in existing memo keys (a skeleton-generation input) — the
   D57 invariant "deleting a workspace is always correct" must hold either
   way; sampled clean rebuilds must not flap.
3. Regression test: a builder RUN executing a script with `#!/usr/bin/env
   bash` (or node) succeeds with the right IMPORTs and fails loudly without.
4. Corpus proof: fetch contexts, re-run `corpus/migrate/echo-server/check.sh
   cix` — the shebang failure should move past the launcher step. Record the
   honest outcome in its receipt + corpus LOG (it may still fail later for
   other reasons; do not oversell — update the receipt to whatever is true).
5. Gate: fmt / warning-denied clippy / workspace tests / tour regen + drift +
   determinism twice / vm-dogfood. Exact repros + cleanup in
   crates/cix-cixfile/LOG.md. Commit on this branch when green.
