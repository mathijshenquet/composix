# fhs-interpreter — downloaded native binaries in bare builders

Status: **draft** (2026-08-04, promoted from the corpusgaps sweep:
directus's GAPS.md and the recorded "native-binary/FHS loader gap",
loop-1).

## 1. The problem

Directus's pnpm dependency tree downloads a prebuilt native binary
(sass-embedded) whose ELF interpreter is `/lib64/ld-linux-x86-64.so.2`.
Composix builders are deliberately bare — there is no FHS loader at that
path — so the build fails before producing an item. This is the corpus's
recorded blocker, and it generalizes: every ecosystem that ships prebuilt
native binaries (node-gyp prebuilds, Playwright browsers, protoc release
tarballs, Go toolchain downloads) will reproduce it.

## 2. Prior work

Nix has three answers, in increasing order of ambient magic:

- **patchelf**: rewrite the binary's interpreter/RPATH to store paths
  (`autoPatchelfHook` automates discovery). Explicit, per-binary, honest.
- **buildFHSEnv**: run the build inside a synthesized FHS tree. Broad but
  reintroduces exactly the image-shaped ambient root composix dissolved.
- **nix-ld**: a host-level shim at the FHS loader path. Host state; out of
  scope for hermetic builders.

Notably, the patchelf route may be *expressible today*: a builder can
`IMPORT ${pkgs.patchelf}` and `RUN patchelf --set-interpreter
${pkgs.glibc}/lib/ld-linux-x86-64.so.2 <binary>` offline. What is missing
is (a) a verified recipe (RPATH for shared-library deps is the hard half),
and (b) teachability — no migrator will derive this unprompted.

## 3. Recommendation

Spike first, design second: attempt the directus conversion with an
explicit patchelf recipe (interpreter + RPATH from imported packages). If
it lands, this is a `docs/migrate.md` teaching pattern plus possibly a
small lint ("downloaded ELF with absolute FHS interpreter — see the
patchelf pattern"), not a language change — the cheapest honest outcome.
Only if the recipe proves impractical (deep transitive library trees,
ecosystems that re-download at runtime) does a language-level mechanism
(`FIXUP`-style directive or an FHS-env builder claim) earn consideration;
buildFHSEnv-by-default is rejected either way as ambient-root regression.

## 4. Open questions

- Who runs the spike: the directus regeneration (once the artifact-import
  decision lands) is the natural vehicle — fold it in, or run standalone?
- If a directive does prove necessary, is it builder-scoped
  (`PATCHINTERP <path>`) or a claim (`CLAIM fhs-build`)? (Leaning: decide
  only with the spike's evidence in hand.)
- Runtime side: a SERVICE shipping a patched binary needs its RPATH
  closure in the item — does the closure walk already catch patched-in
  store references? (Believed yes — patchelf writes real store paths —
  verify in the spike.)
