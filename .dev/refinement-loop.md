# The refinement loop — house method for prompt-distilled translation

Promoted from track-migrate (Mathijs, 2026-07-30). The general pattern:

- **Task T**: translate a wild source artifact into a cix artifact
  (Dockerfile→Cixfile · compose.yml→cix.json · k8s manifests→cix.json tree).
- **Prompt P**: a README teaching T to a fresh reader. P doubles as user docs.
  P may contain only GENERAL lessons (per-artifact fixes = overfitting, forbidden).
- **Loss**: a dual harness — run the source artifact in its native runtime and the
  translation under cix, same probe body, receipt = both transcripts. Loss is
  reported SPLIT: *prompt-loss* (the README failed the converter) vs
  *capability-loss* (cix lacks a mechanism — product backlog, not prompt failure).
- **Loop**: batches in powers of two; a FRESH converter agent per batch that sees
  ONLY P + the source artifact + a cix binary (the validity condition); a verifier
  (≠ converter) triages every failure and adversarially audits a sample of passes
  for vacuous probes; refine P with the smallest general lesson; run deletion
  experiments when loss allows. Track (loss, |P|) per round.
- **Convergence**: prompt-loss ≈ 0 with stable P; capability-loss remaining is
  design input, recorded with corpus row citations.
- **Loss curriculum**: the probe may grow teeth over rounds (v1: central-function
  probe; later: state survives restart, all declared ports serve). Escalate the
  check, keep it minimal per round.
- **Receipts are living**: they pin a cix version; re-run them on language/runtime
  changes — the corpus doubles as a regression suite for cix itself.
- **Robustness round**: late in a track, one batch with a weaker model (luna) grades
  whether P survives a stricter reader.

Instantiations: `track-migrate.md` (running) · compose.yml→cix.json (source runtime:
docker compose, present on host; richer loss: cross-service wiring must work) ·
k8s→cix.json (OPEN CHOICE: source-side receipts need a kind/k3s cluster — heavier
harness — or round 1 runs target-side receipts only and the receipt is honestly half).
