Generated: migrate.md@6ccf252 · terra · 2026-07-30
Status: current

- Copying the built binary into `/bin` and invoking it by a bare name relies on implicit self-import rather than a declared artifact tool contract. → language ([artifact-import draft](../../../cips/draft/artifact-import.md))
- The historical HTTP receipt consumed an unpinned/moving clone and has not been reproduced from the recorded source for the closed-root audit. → evidence
- Upstream copies timezone and CA-certificate trees into its scratch runtime, while the conversion silently omits both. → case
- nixpkgs packages Whoami, but this source-build conversion has no dissolved twin for side-by-side comparison. → case: add dissolved twin
