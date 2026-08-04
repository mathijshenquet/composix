# fixup-elf — IMPORT-wired ELF auto-patching (deferred companion to CIP-95)

> **Deferred** (2026-08-04, Mathijs): CIP-95's path-based FHS surface is
> the chosen mechanism and builds first. This shape returns if/when the
> CIP-94 boundary bites: the path surface lives in cix's constructed
> mount namespace, which a plain nix derivation cannot reproduce, so
> FHS-consuming builders are outside `buildCixfile`'s reproducible set.
> Patched ELFs rebuild fine inside nix — auto-patching is the
> emit-compatible companion when that matters in practice.

The deferred mechanism (CIP-95 v4, spike-verified on directus): when
trace-driven detection (CIP-95 §(E)) identifies a downloaded ELF whose
loader/SONAMEs the *imported* packages provide, cix patches
interpreter+RPATH from the imported set, records the changed file in the
lock, and re-runs the step; unmatched needs refuse loudly (musl-safe).
Open question preserved from v4: bare `FIXUP` opt-in as mutation-mark
versus zero-keyword (Mathijs leaned full magic; the four
arguments-against are recorded in CIP-95's git history at v4).

Also parked here: the taught patchelf RUN pattern stays available today
regardless (CIP-95 §(A)) — this deferral is only about the *automatic*
variant.
