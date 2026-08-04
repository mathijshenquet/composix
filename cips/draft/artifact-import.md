# artifact-import — one canon for referencing package binaries in artifacts

Status: **draft** (2026-08-04, from Mathijs's corpus review: LINK/COPY slop
across the migration corpus; loop-1 promotion of `→ language` gaps).

## 1. The problem

An artifact (SERVICE/APP/ITEM) that runs binaries from nixpkgs packages has
no canonical way to say so. The corpus exhibits **four** competing styles:

1. **LINK into `/bin` + bare START** (adminer, directus, echo-server,
   verdaccio, watchtower, whoami): `LINK ${pkgs.nodejs_22}/bin/node /bin/node`
   then `START node …`. The bare name resolves through the artifact's own
   assembled `/bin` — an *implicit self-import* that reads as magic, and at
   scale becomes a LINK pile (wallos carries six).
2. **Interpolated START path, no assembly at all** (redis):
   `START ${pkgs.redis}/bin/redis-server …`. Reads cleanest for a one-binary
   service, but the resulting item has no `bin/` to speak of — the filesystem
   a debugger sees is oddly empty.
3. **Whole-package COPY + hand-built PATH** (tomcat):
   `COPY ${pkgs.coreutils} /coreutils` … `ENV PATH = /coreutils/bin:…`.
   Materializes package trees at ad-hoc root paths and reimplements what
   IMPORT already does for builders.
4. **Checked-in wrapper scripts that assume style 1** (wallos
   `setup.sh`/`start.sh` invoking bare names).

## 2. Prior work

Builders already solved this: `IMPORT` unions the packages' `bin`, `etc`,
and `share` trees at `/bin`, `/etc`, `/share`, earlier imports win
collisions, and bare command words are checked at build time. Artifacts were
deliberately excluded when IMPORT landed (D56-era) on the theory that the
runtime toolset should be assembled explicitly — and `docs/migrate.md`
teaches exactly that ("The runtime toolset is the artifact's own `bin/`"),
which is where the LINK piles come from: every needed binary becomes a LINK
line, and a reviewer can't tell a deliberate single-binary exposure from a
mechanical pile.

A sibling problem hides in builders today: directus performs a symlink
ritual (`mkdir -p … && rm -rf … && ln -s /var/lib/directus/database
database`) purely to redirect app-relative paths into state — a state
contract buried in shell.

## 3. Recommendation

**(a) Extend `IMPORT` to artifact blocks** with builder semantics: union the
named packages' `bin`/`etc`/`share` into the artifact tree; earlier wins;
bare `START`/`START_PRE` words keep being build-time checked. Wallos's six
LINKs become

```dockerfile
SERVICE wallos
  IMPORT ${pkgs.bash} ${pkgs.coreutils} ${pkgs.php} ${pkgs.nginx} ${pkgs.supercronic}
```

and the self-import stops being magic: the toolset is one visible line,
symmetric with builders.

**(b) LINK keeps a narrower, honest job**: placing a *single file* at a
chosen path — `LINK ${pkgs.nginx}/conf/mime.types /etc/nginx/mime.types` —
or deliberately exposing exactly one binary without the package's whole
`bin/`. A LINK pile (≥3 links into `/bin` from packages) becomes a lint
suggesting IMPORT.

**(c) Runtime-path LINK targets**: artifact `LINK` accepts an absolute
runtime path as target when (and only when) that path lies under one of the
artifact's declared role dirs or a declared `DIR`:

```dockerfile
SERVICE directus
  STATEDIR /var/lib/directus
  LINK /var/lib/directus/database /directus/database
```

The role-dir restriction keeps the link typed — no dangling escapes into
undeclared host paths — and dissolves the mkdir/rm/ln ceremony.

On landing: `docs/migrate.md` rewrites its runtime-toolset section around
IMPORT; LINK piles and whole-package COPY+PATH become anti-patterns;
exhibiting corpus cases flip stale (adminer, directus, echo-server,
filestash, memcached, nats, tomcat, verdaccio, wallos, watchtower, whoami at
minimum).

## 4. Open questions

- **The one-binary canon** (Mathijs explicitly wants a chat round): does
  redis-style direct interpolation in `START` stay *preferred* for
  single-binary services (fewest moving parts, empty-looking item), or is
  canon a one-line `IMPORT ${pkgs.redis}` for uniformity and a debuggable
  tree?
- Does ITEM get IMPORT too? (Pure assembly of a tool tree is a legitimate
  ITEM use; no manifest interaction.)
- Is the LINK-pile lint a warning or a hard error?
- For (c): is "under a declared role dir" the right boundary, or should a
  bare undeclared absolute target be a loud error with a suggestion?
