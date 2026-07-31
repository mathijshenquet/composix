# track/promptrefresh — docs/migrate.md full rewrite in the post-D62 language

Read AGENTS.md first. Context: docs/migrate.md is the migration prompt — it
teaches a model (and humans) how to translate wild Dockerfiles into Cixfiles.
Corpus round r4 proved prompt-rot is a real failure class: the prompt taught
pre-D57 language and migrations failed on vocabulary alone. The prompt gets the
living-receipts treatment: on every language change it must be rewritten and
re-verified, not patched.

Authority: docs/design.md D47–D62 (read them; on conflict design.md wins) and
docs/cixfile.md (current directive reference). Scope: docs/migrate.md ONLY —
`git diff --name-only` must show nothing else (LOG excepted).

## Rewrite requirements
1. **Current language throughout.** Blocks & binders (D47), `FROM … AS` for
   universes and sources, `IMPORT ${pkgs.x}` as builder provisioning (D58 —
   PATH is dead; no implicit cacert: `IMPORT ${pkgs.cacert}` where TLS is
   fetched), `FETCH` with `EXPECT` pins (D56), builder `ENV NAME = value`
   (D59a), quote-aware `EXEC`/`SETUP` argv — `EXEC nginx -g 'daemon off;'` is
   now expressible (D59b), `GRANT jit|egress` (D60), the role-dir family
   STATEDIR/CACHEDIR/RUNDIR/LOGSDIR/CONFIGDIR (D52 complete), `#` comments
   (D53), COPY-dir preference + RUN heredocs (D51). ITEM and SCRIPT do not
   exist (D50/D55); FILE only where store-path-embedding is the actual lesson.
2. **D62 naming flow.** SERVICE block names are the real member names; teach
   `cix build .` (JSON member map, tags nothing), `cix build .#member` (bare
   store path), `-t <tag>` tag-only + repeatable, `--namespace` for
   multi-artifact families, refs always carry an explicit `:tag` (there is no
   implicit `:latest` — say so where Docker muscle memory expects it).
3. **The "everything bare inside builders" lesson** (r4's consistency finding):
   inside a BUILDER, commands are bare names resolved via IMPORT — never
   `${pkgs.x}/bin/x` inside RUN; interpolated absolute paths belong in
   artifact-block directives (EXEC/LINK/COPY sources).
4. **Docker mechanism table**: update every mapping row to current spellings
   (apt/apk/vendor-repo → IMPORT from `${pkgs}`; EXPOSE → PORT; VOLUME →
   STATEDIR; USER/gosu/su-exec/tini → dissolved by DynamicUser/systemd;
   HEALTHCHECK → health per D48(c); docker-socket class → honest ❌ per the
   corpus boundary rows). Keep the honest-gaps section honest — do not
   oversell.
5. **Verified samples.** Every complete Cixfile sample in the doc must be
   proven against the real binary: place it in a scratch dir (git-ignored, use
   .dev/scratch/promptrefresh/, verify with `git check-ignore`) with minimal
   context files and run `target/debug/cix build` (build cix first:
   `devenv shell -- cargo build`). Samples that deliberately show fragments
   must be visibly marked as fragments. Record every verification command and
   result in the LOG. FETCH-bearing samples may pin tiny fetches (keep them
   fast); if a sample cannot be verified, rewrite it until it can or demote it
   to a marked fragment.

## Gate
`git diff --name-only` = docs/migrate.md only; all sample verifications green
and recorded; `devenv shell -- cargo test --workspace` untouched-green as a
smoke that nothing else moved. LOG: append to crates/cix-cixfile/LOG.md
(timestamped, exact repros). Commit on this branch when green.
