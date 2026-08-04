# granular-degradation — one rejected directive must not cost the whole sandbox set

Status: **draft** (2026-08-04, from the tour-CI cascade; promoted out of
open-questions per Mathijs's redo).

## 1. The problem

When a systemd user manager rejects a single hardening directive, cix
retries the unit with an entire property set removed. Concretely, on
GitHub's CI runners the user manager predates `PrivatePIDs=`; cix's
fallback then dropped `PrivateUsers`, `ProtectSystem`, `ProtectHome`,
`PrivateTmp` AND `BindPaths` together — so one unknown key cost the
mount projection, and a service that runs fine on a newer manager fails
there. The degraded development path (the early "rootless is allowed
but weaker" decision, D13 in docs/design.md) was meant as the floor,
not the first fallback.

## 2. Prior work

systemd itself reports exactly which assignment it refused ("Unknown
assignment: PrivatePIDs=yes"), so the information for a precise retry
exists. The current all-or-nothing set dates from when the fallback had
one consumer (local dev).

## 3. Recommendation

Parse the manager's rejection, drop only the named directive, retry;
repeat until the unit loads or a floor set is reached. Log each dropped
directive loudly. The tour/CI environment class (older manager, no
userns) becomes a regression scenario.

## 4. Open questions

- Is per-directive retry cost acceptable (one manager round-trip per
  rejected key), or should cix probe manager capabilities once and
  cache per manager version?
- Does the floor stay exactly D13's current degraded set?
