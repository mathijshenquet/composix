# track/runfixes — runtime-side defect pair (CONFIGDIR path freedom, localhost)

Read AGENTS.md first (gate convention; synchronous receipts). Work in
the herdr worktree on branch `track/runfixes`. Keep
`crates/cix-run/LOG.md` current (dated heading; commit it). Both items
live in docs/open-questions.md "Open for agents" — read those entries;
they carry the verified evidence.

## 1. CONFIGDIR is not path-free (verified triple defect)

`cix build` accepts `CONFIGDIR /config/x`; `cix run` refuses ("config
directory /config/probe must be under /etc"); docs teach role-dir path
freedom. Fix: give CONFIGDIR the same arbitrary-path mirror machinery
STATEDIR already has (CIP-82: mirror the full path under the
unit-scoped systemd directory root, bind back). If a genuine systemd
`ConfigurationDirectory=` semantic blocks that (its root is /etc by
definition — the mirror+bind approach should absorb this exactly like
StateDirectory's /var/lib root does for arbitrary STATEDIR paths), make
an honest STOP report instead of improvising. Either way build and run
MUST agree afterwards: if a restriction survives, it becomes a spanned
build-time error and docs/migrate.md's path-freedom sentence gets
corrected in the same track. Regression: extend the appropriate VM
scenario with a non-/etc CONFIGDIR case; the caddy corpus GAPS entry
flips when the fix truly frees the path (grep for the open-item
citation).

## 2. No `localhost` in the service sandbox

Docker injects /etc/hosts; cix services get none. Adopt the one-blessed-
skeleton-file precedent (/usr/bin/env alias): the runtime sandbox
provides a minimal /etc/hosts (127.0.0.1/::1 localhost) unless the item
supplies its own /etc/hosts (item wins — check assembly collision
order and make it deterministic). Version the skeleton constant.
Regression: scenario asserting localhost resolution in a plain service
+ item-override wins. The caddy GAPS hosts-FILE bullet flips to
resolved; remove the hand-written hosts FILE from the caddy corpus
case? NO — corpus edits are out of scope (fence); flip only the GAPS
Status line to stale-regenerate annotation per the AGENTS extension
rule if applicable.

FENCE: track/tourpolish (tour harness), track/corpusk8s (corpus/ moves,
scenarios roster) run concurrently. Do not touch corpus/ content,
docs/tour/, crates/cix-build (track/buildfixes owns it). Your domain:
crates/cix-run, crates/cix-compose if the unit generation lives there,
nix/scenarios/ focused additions coordinated as small diffs, docs
rows/open-questions updates, your LOG.

## Gate

Standard agent tier + the focused scenarios you touch (df-guard before
VM work; bounded). Synchronous receipts.
