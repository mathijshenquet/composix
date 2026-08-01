# Bikeshed: the `EXEC` keyword

Status: **CIP-80, adopted 2026-08-01** — decided AGAINST §3's
keep-`EXEC` recommendation: the directive becomes **`START`** (see §5).
Spun off from the CIP-79 (health) review, where Mathijs flagged: "I'm
also not sold on EXEC, docker calls it CMD or ENTRYPOINT iirc? what is
the prior art here?"

## 1. The problem

The directive that names a service's argv is spelled `EXEC`
(`EXEC proj1-api`, no shell — D55). Is that the right keyword, given
that every docker migrant arrives knowing `CMD`/`ENTRYPOINT`?

## 2. Prior work

- **Docker**: `ENTRYPOINT` + `CMD`, two interacting directives with
  four combination rules and a shell-form/exec-form footgun each
  (`CMD foo bar` silently wraps in `/bin/sh -c`). The pair exists to
  support image *inheritance* (base sets ENTRYPOINT, child sets CMD) —
  a mechanism composix rejects wholesale (no layer inheritance, D-class
  refusals). Migrating means collapsing the pair anyway.
- **Kubernetes**: `command` + `args` — a deliberate *rename* of
  docker's pair (command overrides ENTRYPOINT, args overrides CMD),
  widely cited as one of k8s's most confusing naming choices precisely
  because it shadows docker's words with different semantics.
- **Compose**: `command` (overrides CMD), `entrypoint` (overrides
  ENTRYPOINT) — faithful to docker.
- **systemd**: `ExecStart=` (with `ExecStartPre/Post`, `ExecStop`, …) —
  the `Exec*` family is the substrate vocabulary composix compiles to.
- **Heroku Procfile**: `web: <command>` — no keyword at all.
- **exec(2)**: the syscall the directive is honest about — argv,
  no shell, no interpretation.

## 3. Recommendation

**Keep `EXEC`.** Three reasons. (a) *Honesty*: the directive does
exactly exec(2) — argv in, no shell (D55); `CMD` would import docker's
expectation of shell-form and of ENTRYPOINT-interaction, both of which
we refuse, so borrowing the word buys recognition of the wrong
semantics. (b) *Substrate alignment*: it compiles to `ExecStart=`, and
sibling directives (`SETUP` → `ExecStartPre`-class) stay in one family.
(c) The k8s precedent shows renaming docker's words *without* docker's
semantics is the worst quadrant; a distinct word with distinct
semantics (our position) is the defensible one. Migration cost is a
table row: `ENTRYPOINT`+`CMD` → one `EXEC` line (docs/migrate.md and
docs/docker.md already teach exactly this).

## 4. Open questions

None — single yes/no: keep `EXEC` as recommended, or rename (the only
serious challenger raised is `CMD`).

## 5. Decision

**`START`**, overriding §3. The dialogue exposed the flaw in §3(b):
systemd's `Exec*` is a *family prefix* — the distinguishing information
lives in the suffix (`Start`/`StartPre`/`Stop`/`Reload`), and composix
already names lifecycle moments (`SETUP` ≈ `ExecStartPre`). The
consistent, extensible family is lifecycle verbs; the no-shell/argv
property is uniform across all command directives and should not name
just one. `CMD`/`ENTRYPOINT` are avoided on the confusion argument
alone.

- `EXEC` → **`START`** (compiles to `ExecStart=`, unchanged semantics:
  quote-aware words, no shell, D55).
- `SETUP` → **`START_PRE`** (verified a pure pass-through to
  `ExecStartPre=`, crates/cix-run/src/unit.rs:280 — nothing special was
  lost). Future `STOP`/`RELOAD` slot into the same family if demanded.
- Shell form stays explicit and always available:
  `START ${pkgs.bash}/bin/sh -c '…'` — the shell is a named dependency
  (it must be, to exist in the sparse projection at all), which is
  exactly D55's provenance point. A bare `/bin/sh` does not exist in a
  service's namespace by construction.
- Old spellings `EXEC`/`SETUP` get the standard migration suggestion
  (crunchy boundary); manifest field `exec` renames to `start`, `setup`
  to `start_pre` (v0, D72).

## Changelog

- 2026-08-01: drafted (recommendation: keep EXEC); adopted same day as
  CIP-80 with the opposite decision (START/START_PRE) after dialogue.
