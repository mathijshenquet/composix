# fhs-interpreter — downloaded native binaries in bare builders

Status: **draft v4** (2026-08-04; v3's trace-driven direction blessed —
"(E) sowieso top, (F) ook" — v4 wires repair into IMPORT itself per
Mathijs, carries how-much-magic as the one open decision, and cuts the
per-target directive/claim shapes as strictly less nice).

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

## 3. Recommendation — detection + IMPORT-wired repair

**(E) Trace-driven detection — always on, no keyword.** The step already
runs under the CIP-87 tracer, and that tracer already parses failed
opens (`= -1 ENOENT` results are recognized in `trace.rs` today) — the
spike's whole painful discovery phase was information the sandbox had
and threw away. When a RUN step fails, cix scans the step's trace for
(i) `execve` ENOENT on a path inside the workdir (the directus
`spawn … dart ENOENT` shape), (ii) reads of FHS loader paths
(`/lib64/ld-*`, `/lib/ld-*`), and (iii) failed SONAME opens during
dynamic loading. The error names the failing executable, its ELF
interpreter, and the missing loader/SONAMEs, and matches them against
what the builder's IMPORTed packages (plus the loader→libc map)
provide. Pure diagnosability (D73 spirit); deletes the loop's discovery
half for every ecosystem at once.

**(F) Auto-repair wired into IMPORT.** The providers are already
declared — IMPORT is the declaration — so repair needs no second
package list: when (E) detects a repairable downloaded ELF, cix patches
interpreter+RPATH from the *imported* set, records the changed file in
the lock, and re-runs the step. A need no imported package satisfies is
a loud refusal naming the SONAME (the spike's musl alternate stays
safe: no imported musl, no silent mispatch).

**The one open decision: how much magic.** Mathijs's lean is full
magic — zero keyword, always on. The honest arguments against
zero-keyword, for the record:

1. *Silent mutation of fetched bytes.* Every other assembly action is
   additive; auto-patchelf rewrites content that upstream shipped, and
   with zero keyword the Cixfile carries no textual trace that binary
   rewriting is part of this build. "Why doesn't my binary match
   upstream's checksum" needs a visible cause in the file.
2. *Intended-failure masking.* A step that deliberately probes platform
   support (or a test asserting an exec fails) gets "repaired" into
   different behavior with no opt-out visible at the site.
3. *Default-path complexity.* Failed-step → patch → re-run machinery
   engages on every build by default, not just where downloaded native
   code is a known ingredient.
4. *House style.* The language refuses ambient magic everywhere else
   (explicit `cacert`, no implicit `:latest`, named FROM). Rewriting
   ELF interpreters is the most invasive action in the language;
   a one-word mark keeps it honest.

The compromise that keeps ~all of the magic: a bare, argument-less
builder `FIXUP` — mechanics 100% automatic from the trace and the
import set, but the single word is the mutation-mark and intent gate,
and (E)'s diagnostic ends with "add FIXUP to enable automatic repair".
Recommendation: bare `FIXUP`; if Mathijs still prefers zero-keyword
after these arguments, it is an alpha-reversible call — the mechanics
are identical.

**Interim and escape: the taught RUN pattern.** Until (F) lands —
and permanently, for ELFs the build never execs (a service spawning a
downloaded binary lazily at *runtime* never hits the builder tracer) —
the explicit `IMPORT ${pkgs.patchelf} ${pkgs.glibc}` +
`RUN patchelf --set-interpreter … --set-rpath …` sequence remains
expressible and taught (the spike's verified recipe).

**Rejected shapes** (v2 §3, cut 2026-08-04 as strictly less nice than
(E)+(F), record retained): a manual per-target `FIXUP ELF <glob> WITH
<pkgs>` directive (target discovery is free from the trace; the manual
form's only residual service — runtime-spawned ELFs — is covered by the
RUN pattern above); `IMPORT … FIXUP <glob>` adjacency (dishonest phase
model: the target does not exist when IMPORT executes); a
`CLAIM prebuilt-elf` (claims grant capabilities, not transformations —
vocabulary pollution). Its "remaining unpatched ELF" diagnostic idea
lives on inside (E).

## 4. Open questions

- The magic level: bare `FIXUP` opt-in (recommended above) vs
  zero-keyword always-on (Mathijs's lean) — one taste call, mechanics
  identical either way.
- (E) needs the tracer to *retain* negative results long enough to
  attribute them to the failing step's report — the `-1 ENOENT` parse
  exists; confirm retention cost is trivial.
- Provider matching beyond the imported set (whole-universe SONAME
  suggestions) needs a nix-index-style database per universe rev —
  defer, or build a small cached index?
- Re-run semantics for (F): re-run the whole step after patching
  (simpler, keying-safe) — measure the cost on the directus case.
- Should (F) also cover `--add-needed`/soname rewrites, or is
  interpreter+RPATH the honest v0?
