# fhs-paths — downloaded native binaries in bare builders (né fhs-interpreter)

Status: **CIP-95, adopted 2026-08-04** (v1–v4 same day; adopted at v5
after Mathijs's reframe — provide the FHS paths instead of rewriting
binaries — with build-first priority).

## 1. The problem

Directus's pnpm dependency tree downloads a prebuilt native binary (the
sass-embedded `dart` executable) whose ELF interpreter is
`/lib64/ld-linux-x86-64.so.2`. Composix builders are deliberately bare —
no FHS loader exists at that path — so the build fails before producing
an item. This generalizes: node-gyp prebuilds, Playwright browsers,
protoc release tarballs, downloaded toolchains all ship FHS-linked ELFs.

## 2. Prior work — including the spike evidence (track/fhsspike, 2026-08-04)

Nix's three answers, in increasing ambient magic: patchelf /
`autoPatchelfHook` (explicit, per-binary), `buildFHSEnv` (reintroduces the
image-shaped ambient root composix dissolved — rejected), `nix-ld` (host
state — out of scope).

The spike ran the patchelf route on the real directus case. **Verdict:
expressible-with-pain — the mechanic works today.** Receipts (full detail
in the track/fhsspike branch journal):

- Minimal recipe: add `${pkgs.patchelf}` `${pkgs.glibc}` to the builder
  IMPORT; after the offline install run `patchelf --set-interpreter
  ${pkgs.glibc}/lib/ld-linux-x86-64.so.2 --set-rpath ${pkgs.glibc}/lib
  <pnpm-store-path>/dart-sass/src/dart`.
- The patched binary resolves interpreter and all five `DT_NEEDED`
  libraries solely from the glibc store path
  (`LD_TRACE_LOADED_OBJECTS=1` exit 0); the baseline `spawn … ENOENT`
  disappears; pnpm's subsequent recursive build accepts the altered file
  (no integrity rejection — expected for post-install repair, not proven
  for every ecosystem).
- The pain, measured: the target path embeds
  `.pnpm/<name>@<version>` (breaks on every lock bump); the tree ships a
  *second*, musl-linked alternate (`ld-musl-x86_64.so.1`) that a naive
  "patch all downloaded ELFs with glibc" would misrepair — per-executable
  explicitness is evidence-backed, not taste; and migrators must read
  `DT_NEEDED` sets to pick supplying packages.
- Two byproduct findings filed separately: the run then died on a bare
  `Error: Not a directory` (a diagnosability defect and the actual
  remaining directus blocker), and the lock grew by ~148k lines of step
  observations (scale question for the lock format).

## 3. Recommendation — provide the paths, don't rewrite the bytes

**Why nix patches, and why that reason does not transfer.** nixpkgs
patchelf's culture exists because *store packages* must never depend on
ambient host paths — a purity constraint on artifacts that leave the
sandbox. A cix builder step runs inside a mount namespace cix itself
constructs; nothing ambient exists unless cix puts it there. The
constraint that forced nixpkgs into byte-rewriting simply does not
apply, and the house already proves the alternative shape: the sandbox
skeleton ships a fixed `/usr/bin/env -> /bin/env` alias that dangles
until an IMPORT supplies `env`, with a diagnostic that names the fix.

**(G) The FHS surface becomes part of IMPORT's union.** IMPORT
additionally unions the packages' `lib/` at `/lib` (earlier-wins, same
rule as `bin`), and the skeleton gains the well-known loader aliases —
`/lib64/ld-linux-x86-64.so.2` and `/lib/ld-musl-x86_64.so.1` (per-arch
table) — pointing into `/lib`, dangling until a libc is imported,
exactly like the env alias. `IMPORT ${pkgs.glibc}` then makes a
downloaded GNU binary loadable with **no mutation, no detection, no
retry**: the paths exist before the step runs, deterministically
derived from the declaration. What this wins over ELF patching:

1. *No mutation* — upstream bytes stay byte-identical (parity claims
   and checksum comparisons keep working); the whole how-much-magic
   debate dissolves because nothing is silently rewritten.
2. *No loop* — patching needs detect→patch→re-run because targets
   appear mid-step; aliases exist up front. Mathijs's inline-catch
   worry is exactly right, and the answer is to need no catching.
3. *Runtime spawns covered* — a SERVICE that lazily spawns a
   downloaded ELF never hits the builder tracer; artifact IMPORT gives
   it the same loader surface. The patch route structurally missed
   this class.
4. *musl-safe by construction* — each loader path is distinct; the GNU
   alias resolves only if glibc-family is imported, the musl alias only
   for musl. No per-binary classification, no misrepair.
5. *Per-binary work disappears* — no brittle `.pnpm/<ver>` globs.

The mechanism detail the round-2 spike must verify: the aliased nix
ld.so does not search `/lib` by default. Candidate wirings — a
generated `/etc/ld.so.conf`+cache in the union, or a builder-default
library path — where naive `LD_LIBRARY_PATH=/lib` is suspect (it
outranks DT_RUNPATH and could shadow nix-built binaries' own
resolution); the spike picks the wiring that leaves RUNPATH-carrying
binaries untouched.

**(E) Trace-driven diagnostics, always on.** Unchanged from v3/v4 in
mechanism (the tracer already parses `-1 ENOENT`), but its output now
teaches the *declaration*, mirroring the env-alias hint: "dart requires
the FHS loader /lib64/ld-linux-x86-64.so.2 and libm.so.6, libc.so.6 —
IMPORT a package set providing them (${pkgs.glibc})." Where the need is
outside the imported set, the convergence loop is the author adding one
IMPORT line per diagnostic — cix itself never retries.

**(A) The taught patchelf RUN pattern stays** (Mathijs: a fine
works-now solution): the spike-verified
`IMPORT ${pkgs.patchelf} ${pkgs.glibc}` + `RUN patchelf
--set-interpreter … --set-rpath …` recipe remains taught as the
works-today path and the permanent escape for exotic needs (RPATH
surgery, `--add-needed`) that alias semantics cannot express. (E)'s
diagnostic references it until (G) lands.

**Fallback**: if the round-2 spike shows the loader/search wiring
cannot be made clean (RUNPATH shadowing unresolvable, cache generation
too stateful), v4's IMPORT-wired auto-patching returns as the
mechanism, with its bare-`FIXUP` mutation-mark question intact. The
rejected per-target shapes (manual `FIXUP ELF`, IMPORT-adjacency,
`CLAIM prebuilt-elf`) stay rejected on the grounds recorded in v4.

## 4. Open questions

- The ld.so search wiring for `/lib` (spike round 2): generated
  ld.so.conf/cache vs a default library path — measured against the
  RUNPATH-shadowing hazard on nix-built binaries in the same builder.
- Do artifacts always carry the loader aliases (dangling, like
  /usr/bin/env) or only when a libc-family package is imported?
  (Skeleton simplicity vs surface minimalism; env precedent says
  always-dangling is fine.)
- 32-bit (`/lib/ld-linux.so.2`) and other arch aliases: full table now
  or grow on demand?
- Does `lib/` union join builder IMPORT only, or artifacts too from day
  one? (Runtime-spawn coverage argues artifacts; closure size argues
  measuring first.)
- Directus regen is the acceptance case: `IMPORT ${pkgs.glibc}` with
  zero patchelf lines must build (modulo the separate bare
  `Error: Not a directory` defect).

## 5. Decision

Adopted 2026-08-04 (Mathijs: "fhs path based approach lijkt me ideaal,
laten we dat als eerste bouwen"). §3 as written: (G) lib/-union +
skeleton loader aliases, (E) trace-driven diagnostics teaching the
IMPORT, (A) the patchelf RUN pattern taught as escape. Build order:
first — ahead of regen wave 2 (directus is the acceptance case). The
round-2 spike (ld.so search wiring vs RUNPATH shadowing) opens the
implementation track and may still fall back per §3.

Recorded boundary: this approach relies on cix's constructed mount
namespace and is therefore invisible to CIP-94's `buildCixfile` (a nix
derivation cannot create `/lib64/…`). FHS-consuming builders are
excluded from the eval-from-lock reproducible set; if that boundary
ever matters in practice, the deferred FIXUP auto-patching
([deferred/fixup-elf](../deferred/fixup-elf.md)) is the
emit-compatible companion, since patched ELFs rebuild fine inside nix.

## Changelog

- 2026-08-04: v1 (spike mandate) → v2 (four syntax shapes on spike
  evidence) → v3 (trace-driven detection + declared-provider repair) →
  v4 (IMPORT-wired, per-target shapes cut) → v5 (path surface instead
  of byte rewriting) → adopted, renamed fhs-paths, build-first.
