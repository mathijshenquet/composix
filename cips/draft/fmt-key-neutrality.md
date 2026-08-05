# fmt-key-neutrality — formatting must not invalidate build inputs (CIP-light)

Status: **draft** (2026-08-05), exhibited by the HAProxy corpus case.

## Problem

D59 deliberately makes declared builder `ENV` text part of the resolved step
chain, while CIP-87 makes a FETCH replay depend on that resolved step identity
and its captured input snapshot. That is correct for semantic text, but a
formatter's whitespace-only indentation cannot become semantic input. Today it
does: the HAProxy Cixfile is accepted unindented and locked; `cix fmt` indents
the directives, and the same FETCH gets a new identity.

Exact independent reproduction on 2026-08-05:

```text
$ target/debug/cix build /var/tmp/composix-haproxy-fmt.FwbaR9#haproxy
/nix/store/h0n4r9i2c13sciwmv7w26qfzbzgbfj96-cix-item-haproxy

$ target/debug/cix fmt /var/tmp/composix-haproxy-fmt.FwbaR9

$ target/debug/cix build --cold /var/tmp/composix-haproxy-fmt.FwbaR9#haproxy
Error: FETCH builder:build:3-00ba784e8f7b has no locally cached replay snapshot at
/home/mathijs/.cache/cix/fetch-snapshots/3378f6418827b7c769e19aefc1f52f90dce578bebce743ab98056cd4c5e2336d;
run a non-cold build first (--cold never refetches)
```

The unformatted build and the formatted-copy formatter both exited 0; the
formatted cold build exited 1. The only intended change was indentation.

## Proposal

Compute every semantic key from the parser's canonical form (or an equivalent
canonical AST serialization), never from raw declared text. Preserve D59's
meaning: changing an ENV name, value, directive order, imports, command, or
resolved arguments still changes the key. Normalize only syntax that `cix fmt`
is allowed to rewrite: indentation, whitespace between tokens, and canonical
quote/layout representation where the parsed value is identical.

Add a regression fixture that locks and cold-replays a deliberately
non-canonical Cixfile, formats a copy, and asserts identical FETCH identities,
the same snapshot lookup, and the same item output. This is the source-level
dual of CIP-87's constructive-trace invariant: representational changes must
not cause an invented input change.

## Effort

M. Centralize key serialization at the resolved AST boundary, version/fingerprint
the old raw-text identities honestly, and add warm/cold formatter-equivalence
coverage.
