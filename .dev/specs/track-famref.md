# track/famref — D65(a3): cix-item index refs as FROM artifact binders

Read AGENTS.md first. Authoritative: design.md **D65** (read it fully). Scope:
crates/cix-cixfile (+cix-common ref parsing, cix-index resolution), examples
(one small demonstration), docs (cixfile.md FROM section; docs/migrate.md
`COPY --from=<image>` row), tour where FROM is taught. Runs parallel to the
migrate-r5 corpus round — do NOT touch corpus/migrate/**.

1. Grammar: a FROM input token is classified per D65(b): known flakeref scheme
   (`github:`, `git+`, `path:`, `tarball+`, `.`, `./…`) ⇒ flakeref (existing
   paths); otherwise it MUST parse as an index ref with an explicit `:tag`
   (local `name:tag`, family `family/member:tag`, or qualified
   `host/family/member:tag`); otherwise a parse error that names both
   grammars. No default tag — an untagged token that is not a flakeref is the
   D62 `:latest` error.
2. Resolution: resolve the ref via cix-index (local table; qualified refs may
   pull per the existing pull path), verify narHash, and pin in Cixfile.lock
   as `ref → {storePath, narHash}`. `--update-lock <binder>` re-resolves that
   binder (tags may move; the lock pins). Missing-from-index = clear error
   telling the user to pull or tag first.
3. Binder semantics: an artifact binder is a TREE — legal wherever source
   binders are (COPY sources, LINK targets); `${binder}/path` interpolation
   only. It never binds a namespace: attribute-path use (`${binder.attr}`) is
   an error citing D65(c). It is not importable in builders (D65(d) deferral —
   error citing it).
4. Tests (real-nix): tag a locally built item, FROM it by tag in a second
   Cixfile, COPY out of it, build green; lock pins and survives a tag move
   until --update-lock; narHash mismatch = hard error; untagged/unknown-ref/
   attr-use/IMPORT-of-item error cases.
5. Docs: cixfile.md FROM section gains the third input kind with the
   disambiguation rule; docs/migrate.md updates its multi-stage/`COPY --from`
   guidance (cross-IMAGE copy now expressible). Tour: add or extend the
   chapter where FROM/tags meet (keep it small; regen + drift).
6. Gate: fmt / warning-denied clippy / workspace tests / tour regen + drift +
   determinism twice / vm-dogfood. Exact repros + cleanup in
   crates/cix-cixfile/LOG.md. Commit on this branch when green.
