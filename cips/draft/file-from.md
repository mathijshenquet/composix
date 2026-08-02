# FILE … FROM — interpolating file authoring without heredocs

Status: **draft** (2026-08-02, from Mathijs's wallos review: "waarom
geen COPY? Agents blijven heredocs doen").

## 1. The problem

Cixfile authors (agents especially) keep writing large `FILE <path>
<<EOF` heredocs for configs and scripts. The readability cost is real:
file content buried in the Cixfile, no editor tooling, page-long
blocks. The structural cause is that the heredoc is currently the ONLY
channel that both authors a file into the artifact AND interpolates
`${…}` — `COPY` moves bytes verbatim. So even authors who want
file-per-file layout are forced inline the moment one store path
appears in a config (wallos: one `${pkgs.nginx}/conf/mime.types` line
drags a 40-line nginx.conf into the Cixfile).

## 2. Prior work

Docker has no interpolating COPY (people run envsubst/sed in RUN — we
inherited that idiom in wallos's giga-RUN). Nix's `substituteAll` /
`pkgs.replaceVars` is exactly this shape: a source file with
placeholders, substituted at build. The Cixfile already has one
substitution surface with defined semantics: `${…}` in directive
arguments (D32 namespaces, D69d no-functions).

## 3. Recommendation

`FILE <dest> FROM <source>` — author the file as a real file next to
the Cixfile (or from any binder), substitute `${…}` occurrences in its
CONTENT with exactly the Cixfile interpolation semantics (namespaced
attribute paths and binder paths; same errors, same keying: source
hash + resolved substitutions enter the step key per D48a).

- Heredoc `FILE` stays legal but the canon flips: real files + COPY
  (no interpolation) or FILE…FROM (interpolation); heredocs only for
  genuinely tiny content. Canon recorded in migrate.md + cixfile.md.
- Escaping: a literal `${` in the source is spelled `$${` (same rule
  as directive text, one rule everywhere).
- No new computation surface: substitution-only, no conditionals, no
  loops — the D28/D69d boundary (computation lives in .nix or
  generators, never in templates).

## 4. Open questions

1. Spelling: `FILE <dest> FROM <src>` (reads as provenance, matches
   FROM's "source" family) vs `COPY --subst` (flag on COPY; but COPY
   currently promises byte-fidelity, which is a nice invariant to
   keep). Draft recommends FILE…FROM.
2. Should un-substituted `${…}`-looking content in a plain COPY'd file
   warn? (Probably not — COPY is bytes, that's its contract.)
