# pnpm-wall — dependency-store volatility and bounded corpus evidence (CIP-light)

Status: **draft** (2026-08-05).

## Problem

The corpus shows several distinct failures that all look like a “pnpm wall” at
first glance, but are materially different: missing bootstrap prerequisites,
moving registry/cache metadata, stale lock graphs, and snapshot scale. Treating
them as one ecosystem failure would produce bad language changes.

## Five precise exhibits

1. **Homer staging** reached `UNABLE_TO_GET_ISSUER_CERT_LOCALLY`; independent
   recheck with `cacert` and a FETCH-traced pnpm data directory completed the
   offline Vite build. It is a missing-prerequisite false positive, not a wall.
2. **Dozzle**: `pnpm fetch --ignore-scripts` exceeded its independent 300-second
   bound without an item (`corpus/migrate/docker/dozzle/receipt.md`).
3. **Verdaccio**: cold replay observes a volatile pnpm root read set after the
   ordinary monorepo rebuild fails to make an item (`verdaccio/GAPS.md`).
4. **Directus**: offline frozen install stops on an upstream stale root
   `package.json`/`pnpm-lock.yaml` mismatch, before deployment
   (`directus/receipt.md`).
5. **Filestash (adjacent lock-scale control)**: its first Go-module FETCH seals
   about 2.7 GiB and 69k files, exceeding 20 minutes; this is the same
   snapshot-scale shape without pnpm (`filestash/receipt.md`).

## Direction

Keep the distinctions visible in diagnostics and receipts. Record volatile
FETCH trees as TOFU pins only when their consumed output is stable enough to
replay; otherwise report the bounded scale/volatility result. Do not add a
global pnpm exception or weaken network isolation.
