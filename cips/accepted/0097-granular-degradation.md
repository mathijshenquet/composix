# granular-degradation — one rejected directive must not cost the whole sandbox set

Status: **CIP-97, adopted 2026-08-04** (v2 akkoord).

## 1. The problem

When a systemd user manager rejects a single hardening directive, cix
retries the unit with an entire property set removed. On GitHub's CI
runners the manager predates `PrivatePIDs=`; cix's fallback then
dropped `PrivateUsers`, `ProtectSystem`, `ProtectHome`, `PrivateTmp`
AND `BindPaths` together — one unknown key cost the mount projection.
Background: the "degraded development path" is the early decision
(docs/design.md D13) that rootless `--user` runs are allowed but
weaker — no DynamicUser, reduced sandboxing — with production on the
system manager. That degraded set was meant as the floor, not the
first fallback.

## 2. Prior work — is there a capability query? (researched)

systemd has NO per-directive support query: `systemctl show -p
Features`/`--version` expose compile-time flags, not directive
vocabulary. But the practical probe exists and is fast:
`systemd-analyze --user verify <unit>` parses a unit with the TARGET
manager's own parser and prints "Unknown assignment" per unsupported
key WITHOUT starting anything. One verify round-trip yields the exact
rejected-directive set. (Version-keyed tables — the podman/quadlet
approach — are the maintenance surface we avoid.)

## 3. Recommendation

Before first unit submission per manager, run one batched
`systemd-analyze verify` probe over the full directive set cix might
emit; cache the rejected set keyed by manager identity+version. Unit
generation then omits exactly the unsupported directives, loudly
logging each omission. No retry loop in the common case; the runtime
"Unknown assignment" parser stays as belt-and-braces for managers
whose verify output diverges. D13's degraded set remains the explicit
floor for rootless mode, unchanged.

## 4. Open questions

- Cache key: manager version alone, or version+unit-path (container
  managers may differ per instance)?
- Does `systemd-analyze --user verify` exist on every supported
  manager version (it is old, but confirm the oldest CI image)?

## Decision

Adopted 2026-08-04 at v2: one batched `systemd-analyze verify` probe
per manager (rejected set cached by manager identity+version), unit
generation omits exactly the unsupported directives with loud logs,
runtime Unknown-assignment parsing as belt-and-braces, D13's degraded
set as the unchanged floor.
