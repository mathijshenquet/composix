# fhs-interpreter — downloaded native binaries in bare builders

Status: **draft v2** (2026-08-04; v1 promoted from the corpusgaps sweep,
v2 rewritten on the track/fhsspike evidence per Mathijs: spike first,
then concrete syntax proposals).

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

## 3. Recommendation — four concrete shapes

**(A) Taught RUN pattern (expressible today).**

```dockerfile
IMPORT ${pkgs.bash} … ${pkgs.patchelf} ${pkgs.glibc}
RUN pnpm install --recursive --offline --frozen-lockfile
RUN patchelf --set-interpreter ${pkgs.glibc}/lib/ld-linux-x86-64.so.2 \
    --set-rpath ${pkgs.glibc}/lib \
    node_modules/.pnpm/sass-embedded-linux-x64@1.93.3/…/dart-sass/src/dart
```

No language change; lands as a migrate.md teaching section. Cost:
migrators must recognize ELF failures, inspect DT_NEEDED, know platform
variants, and hand-maintain brittle versioned paths.

**(B) `FIXUP ELF` builder directive.**

```dockerfile
FIXUP ELF node_modules/.pnpm/sass-embedded-linux-x64@*/…/dart-sass/src/dart \
    WITH ${pkgs.glibc}
```

Builder-only, runs at its declaration point (after materialization). Cix
validates the target is a dynamically-linked ELF of a known
architecture/libc, sets the interpreter from the named libc package,
derives RPATH from the `lib/` dirs of every `WITH` package, refuses
unknown combinations loudly, and records the changed file in the lock. A
bounded glob absorbs version-bump path brittleness. Multiple `WITH`
packages cover multi-provider DT_NEEDED sets.

**(C) `IMPORT … FIXUP` adjacency.**

```dockerfile
IMPORT ${pkgs.glibc} FIXUP node_modules/**/dart-sass/src/dart
```

Reads well — the satisfying package is visibly adjacent — but the phase
model is dishonest: IMPORT executes before the target exists (it appears
only after `pnpm install`), so the directive would need deferred
execution, breaking IMPORT's current one-shot semantics. Recorded mostly
to reject it for that reason unless the phase story improves.

**(D) Declarative prebuild claim.**

```dockerfile
CLAIM prebuilt-elf node_modules/**/dart-sass/src/dart WITH ${pkgs.glibc}
```

Scales as teaching vocabulary ("this builder contains downloaded native
code") and could power a targeted diagnostic when *other* unpatched FHS
ELFs remain. But the spike's musl-alternate evidence cuts against any
claim-shaped/ambient form doing the repair itself, and claims elsewhere
in the language grant *capabilities*, not transformations — a
transformation wearing a claim costume muddies that vocabulary.

**Lean:** (A) immediately (it is true today and the corpus needs it for
directus regen), (B) as the language feature when a second real case
arrives — it keeps per-executable explicitness, adds validation the RUN
pattern cannot (arch/libc refusal, RPATH derivation, lock recording),
and its glob absorbs the measured brittleness. (C) rejected on phase
grounds; (D) rejected as claim-vocabulary pollution, though its
"remaining unpatched ELF" diagnostic idea can ride along with (B).

## 4. Open questions

- Is one real case enough to build (B) now, or does (A) hold until a
  second ecosystem (Playwright? node-gyp?) lands in the corpus?
- Glob semantics in (B): bounded segment wildcards only (`@*`), or full
  `**`? (Brittleness vs precision — the spike argues for the narrowest
  glob that survives a lock bump.)
- Should (B) also cover `--add-needed`/soname rewrites, or is
  interpreter+RPATH the honest v0?
- The musl alternate: refuse-by-default is decided by the evidence, but
  should the refusal message name the musl loader explicitly to teach
  the variant problem?
