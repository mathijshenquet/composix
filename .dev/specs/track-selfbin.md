# track/selfbin — D64: implicit self-bin runtime PATH + bare EXEC resolution

Read AGENTS.md first. Authoritative: design.md **D64** (read it fully). Scope:
crates/cix-cixfile (+cix-run for the runtime env default and exec/debug env),
examples, docs (cixfile.md, migrate.md where EXEC/PATH-history is taught), tour.
SEQUENCED AFTER track/demofix merges (same example/tour surface).

1. Runtime default: generated SERVICE/APP manifests carry `PATH=<item>/bin` as
   the env default when the Cixfile declares no `ENV PATH`; an explicit
   `ENV PATH = …` replaces it entirely (test both). The default appears only
   when the item actually has meaning for it — decide-and-document whether to
   emit unconditionally or only when bin/ exists in the assembled item; prefer
   unconditional (simpler, harmless) unless a test shows harm.
2. Bare `EXEC <name>` / `SETUP <name>` (single word, no `/`, no `${…}`):
   build-time resolve against the item's own bin/ per D31 item-relative
   mechanics; write the resolved item-relative path into the manifest exactly
   as `EXEC bin/<name>` would. Not found = error listing the item's bin/
   entries. `EXEC bin/x` and `EXEC ${pkgs.x}/bin/x` unchanged.
3. `cix exec` / `cix debug` reconstructed environments inherit the same PATH
   default (check crates/cix-run exec env assembly).
4. Manifest/version: if this adds or changes a manifest field, gate per D15;
   if it is purely a generated-env default, say so in the LOG and leave the
   version alone.
5. Sweep: docs/cixfile.md EXEC/env sections; docs/migrate.md rows that teach
   EXEC (keep its fragments honest — bare EXEC of your own binary becomes the
   preferred spelling); examples where `EXEC bin/x`-style spellings can become
   bare; tour regen.
6. Gate: `devenv shell -- cargo fmt --all --check`; warning-denied workspace
   all-target clippy; `cargo test --workspace`; tour regen + drift +
   determinism twice; `devenv shell -- nix build .#checks.x86_64-linux.vm-dogfood --no-link -L`.
   Exact repros + unit cleanup in crates/cix-cixfile/LOG.md. Commit on this
   branch when green.
