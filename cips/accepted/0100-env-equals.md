# CIP-100: env-equals — switch ENV to the `NAME=value` grammar (CIP-light)

Status: **accepted** (2026-08-05; CIP-light. Drafted 2026-08-04 from
Mathijs's dispositions review: "willen we niet liever ENV NAME=value?
wat is prior work?").

**Prior work (the answer): unanimous.** POSIX/bash assignment is
`NAME=value` with NO spaces (spaces break it); dotenv files, docker
`ENV k=v`, compose `environment:` entries, systemd `Environment=
"k=v"`, and make all use the same form. Our spaced `ENV NAME = value`
is the ecosystem outlier — the original disposition (keep spaces, hint
on `=`) defended the wrong incumbent.

**Proposal.** Canon becomes `ENV NAME=value`. The full coherent set:

```dockerfile
ENV PORT=8080            # default value
ENV PORT=8080 required   # operator must supply (overrides default? no:
                         # required forbids a default — parse error)
ENV API_TOKEN required   # mandatory, no default
ENV ADMINER_DESIGN       # optional, no default (CIP-96 bare form)
```

The spaced form becomes a parse error with the reversed hint ("write
`ENV NAME=value`"). Values containing spaces use quotes, exactly like
bash/systemd. Migration: mechanical sweep of corpus/examples/tour in
the implementing track (alpha, no compat).

**Effort.** Small grammar change + repo-wide mechanical sweep.

## Decision

Adopted as proposed (Mathijs, 2026-08-05: "mint hem maar, of amendeer
een bestaande"). Minted standalone next to CIP-96 rather than amending
it: the grammar switch and the optionality semantics stay separately
citable; one implementation track may still land both.

Changelog:
- 2026-08-05 — adopted as CIP-100.
