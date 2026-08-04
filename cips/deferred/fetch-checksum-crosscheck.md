# fetch-checksum-crosscheck — verify observed fetches against ecosystem-declared checksums

> **Deferred** (2026-08-04, split out of CIP-94 at adoption).

The idea: at lock time cix holds the fetched bytes while the source tree
often declares per-dependency checksums (Cargo.lock sha256s,
package-lock integrity fields). Cross-check observation against
declaration; refuse on mismatch.

The steelman that survived Mathijs's "it can be wrong in two places"
objection: the two values have different custody at different times — a
registry/CDN tamper today fails against a checksum upstream committed
months ago, collapsing our lock-time network-TOFU into the
already-assumed source trust (whose build.rs we execute anyway).

Why deferred anyway: the check requires per-ecosystem lockfile parsing,
which contradicts the ecosystem-blindness CIP-94 fixed (one FOD per
FETCH, no ecosystem awareness through any back door). Returns only if a
real supply-chain scare makes the hardening worth that cost.
