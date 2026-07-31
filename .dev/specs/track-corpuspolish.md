# track/corpuspolish — rewrite the migration corpus in the current language

Read AGENTS.md first. Authoritative for the language: docs/design.md D56–D64
and docs/migrate.md (freshly rewritten — it IS the style guide; follow it).
Scope: corpus/migrate/** only. Contexts are fetched, not vendored: run
`cd corpus/migrate && ./fetch.sh --all` first.

## Problem
All 13 corpus pairs predate D58: builder `PATH` lists, service `PATH bin`,
`${pkgs.x}/bin/x` inside RUN/FETCH, `SSL_CERT_FILE=` cacert dances, and
pre-D62 check.sh build captures. None of them parse with today's cix. They are
the living receipts of the migration story and must speak the living language.

## Work
1. **Rewrite every candidate Cixfile** in current language:
   - builder `IMPORT ${pkgs.x} …` replaces PATH lists; ALL commands bare
     inside builders (the migrate.md rule); `IMPORT ${pkgs.cacert}` replaces
     SSL_CERT_FILE dances; `FETCH` with `EXPECT` pins where the fetch output
     is stable (adminer's tarball, etc. — the queue note: "adminer EXPECT");
     builder `ENV NAME = value` where repeated env prefixes existed (D59a).
   - service/app blocks: assembled-own-`bin/` + bare `EXEC <name>` per D64
     (`COPY ${build}/watchtower bin/watchtower` + `EXEC watchtower`); GRANT
     spellings where JIT/EGRESS-era forms existed; role-dir directives current.
   - Do NOT change what a pair honestly cannot do (dozzle/watchtower stay
     docker-socket ❌ boundary rows; echo-server's npm-timeout class stays
     honest — modernize the spelling, keep the verdict truthful; its multi-
     FETCH retry is migrate-r5 scope, not yours).
2. **check.sh scripts to the D62 contract**: bare `cix build <dir>` now prints
   a JSON member map — use the member selector (`build "<dir>#<member>"`) for
   path captures, keeping the existing fetch-guard and probe logic.
3. **Fresh cix-side receipts**: for every pair, run `./check.sh cix` after a
   fresh fetch and record the result in its receipt.md (dated 2026-07-31,
   language generation noted). Pairs that previously passed cix must pass
   again; a regression = investigate before writing the receipt. Docker-side
   receipts are NOT rerun — keep their original dates and say so explicitly
   in each receipt (cix side refreshed, docker side historical).
4. Update corpus/migrate/LOG.md with exact commands and per-pair outcomes.

## Gate
`cargo build` for the cix binary; every `./check.sh cix` result recorded
honestly (green where previously green); `git diff --name-only` confined to
corpus/migrate/**; workspace tests untouched-green as smoke
(`devenv shell -- cargo test --workspace`); no context/ trees committed
(`git ls-files 'corpus/migrate/*/context/**'` stays empty); test-created
units stopped and reset. Exact repros in corpus/migrate/LOG.md. Commit on
this branch when green.
