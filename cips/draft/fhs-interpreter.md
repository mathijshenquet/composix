# fhs-interpreter — downloaded native binaries in bare builders

Status: **draft v3** (2026-08-04; v1 promoted from the corpusgaps sweep,
v2 rewritten on the track/fhsspike evidence, v3 after Mathijs's
structural direction: the build already runs traced in a sandbox — detect
failed FHS reads there and match them to providers, instead of teaching
humans a "try build → patch +1" loop; repair stays opt-in via a keyword).

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

**(E) Trace-driven detection — always on, no keyword.** The step already
runs under the CIP-87 tracer, and that tracer already parses failed opens
(`= -1 ENOENT` results are recognized in `trace.rs` today) — the spike's
whole painful discovery phase was information the sandbox had and threw
away. When a RUN step fails, cix scans the step's trace for (i) `execve`
ENOENT on a path inside the workdir (the directus `spawn … dart ENOENT`
shape), (ii) reads of FHS loader paths (`/lib64/ld-*`, `/lib/ld-*`), and
(iii) failed SONAME opens during dynamic loading. The error then names
the failing executable, its ELF interpreter, and the missing
loader/SONAMEs, and matches them against what the builder's IMPORTed
packages (plus the loader→libc map) provide:

```text
error: RUN step 6 failed: node_modules/…/dart-sass/src/dart requires the
  FHS loader /lib64/ld-linux-x86-64.so.2 (not present in a cix builder)
  and libm.so.6, libc.so.6 — provided by imported ${pkgs.glibc}.
  Downloaded native binaries need an explicit repair: see FIXUP.
```

Pure diagnosability (D73 spirit): no semantics change, no opt-in needed,
and it deletes the loop's discovery half for every ecosystem at once.

**(F) Opt-in bounded auto-repair — the keyword.**

```dockerfile
FIXUP WITH ${pkgs.glibc} ${pkgs.zlib}
```

Builder-scoped, opt-in. With the declaration present, a RUN step that
(E)-detects a repairable downloaded ELF gets it patched automatically —
interpreter and RPATH — **only from the declared WITH set**: cix maps the
observed missing loader/SONAMEs to the declared packages' `lib/` trees,
patches, records the changed file in the lock, and re-runs the step. A
need no declared package satisfies is a loud refusal naming the SONAME
(the musl alternate stays safe: no declared musl, no silent mispatch).
The human loop collapses to "read the (E) diagnostic, declare the
providers it names"; the iteration moves inside one build step, bounded
by the declared set instead of ambient search.

**Lean (v3):** (E) unconditionally — it is trace plumbing we largely
have. (F) as the language feature, replacing v2's manual-target (B):
target discovery came free from the trace, so the human declares only
*providers*, which is exactly the dependency fact worth writing down.
(A) remains the interim teaching until (F) exists; (B)'s validation
machinery (arch/libc refusal, RPATH derivation, lock recording) survives
inside (F) as implementation; (C) rejected on phase grounds; (D) rejected
as claim-vocabulary pollution, though its "remaining unpatched ELF"
diagnostic rides along in (E).

## 4. Open questions

- (E) needs the tracer to *retain* negative results long enough to
  attribute them to the failing step's report — today the `-1 ENOENT`
  parse exists but negatives are presumably dropped after read-set
  filtering. Confirm the retention cost is trivial before promising
  "always on".
- Provider matching scope: the WITH set and IMPORTed packages are cheap
  to search; suggesting providers from the whole locked universe needs a
  nix-index-style SONAME database — defer, or build a small cached index
  per universe rev?
- Does (F) re-run the step after patching, or patch-and-continue within
  the failed step? (Re-run is simpler and keying-safe; measure the cost
  on the directus case.)
- Keyword bikeshed: `FIXUP WITH …` vs `FIXUP ELF WITH …`; and does a
  manual per-target form (v2's B) stay as an escape hatch or is
  provider-only declaration enough?
- Should (F) also cover `--add-needed`/soname rewrites, or is
  interpreter+RPATH the honest v0?
- The musl alternate: refusal is decided; should the message name the
  musl loader explicitly to teach the variant problem?
