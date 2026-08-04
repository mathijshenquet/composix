# Kubernetes corpus axis

Each future case directory will keep its pinned Kubernetes manifests in place of a
Dockerfile, plus `SOURCE`, `GAPS.md`, `receipt.md`, and a `check.sh` with the same
executed-receipt conventions as the Docker axis. Its conversion target is the
equivalent Cixfile or Cixfiles and a `compose.json` where the manifest describes
multiple cooperating workloads.

This is only the authoring skeleton. No Kubernetes manifest has been converted
yet; candidate selection and verdict shape live in `CANDIDATES.md`.
