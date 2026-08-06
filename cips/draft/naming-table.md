# naming-table — one name for everything, decided before 0.1

Status: **draft** (2026-08-06; requested by Mathijs. Earlier rounds
explored psychopomp/katharsis/pyxis families under systemd/nix
aesthetics; Mathijs 2026-08-06: "ik leun naar alles composix noemen" —
this draft works that lean out into a decidable table.)

## 1. The problem

The project is `composix`, the binary is `cix`, files are `Cixfile`,
units are `cix-*.service`, crates are `cix-*`, env vars are `CIX_*`,
store items are `cix-item-*`. Two name families coexist without a
recorded decision, and every new surface (diagnostics, docs, k8s
bridge) has to pick one. Renames are epoch-cheap today (alpha, one
repo, corpus sweep already scheduled) and painful after 0.1.

## 2. Prior work

- **Long project + short binary is the industry norm**: kubernetes/
  kubectl, terraform/tf (community), ripgrep/rg, prometheus/promtool.
  Users type the short form thousands of times.
- **Same-name-everywhere is the other norm**: docker/docker/Dockerfile,
  cargo/cargo/Cargo.toml, git/git — brand unity, zero mapping cost.
  Notably these are all ≤2 syllables.
- **systemd unit prefixes** favor the service's own name (`docker.service`,
  not `dkr.service`) — unit names are read, not typed.
- **nix** ships `nix-*` tools under the exact project name.
- The abandoned invented families (psychopomp, katharsis, pyxis):
  memorable but explain-nothing names; every one of them costs a
  sentence of introduction that "composix" gets for free (compose +
  nix is self-describing). Recorded as rejected-lean.

## 3. Recommendation — the table under "everything composix"

`composix` is 3 syllables / 8 chars — typing cost is real for the CLI.
The table separates TYPED surfaces (short form defensible) from READ
surfaces (full name wins). Mathijs's lean applied maximally, with the
one typed exception flagged as the open call:

| Surface | Today | All-composix | Note |
| --- | --- | --- | --- |
| Project / repo | composix | composix | unchanged |
| CLI binary | `cix` | `composix` (+ optional `cix` alias) | THE open call — see §4 |
| File | `Cixfile` | `Composixfile` | reads fine, types rarely |
| Lock | `Cixfile.lock` | `Composixfile.lock` | follows file |
| Manifest key | `cixManifest` | `composixManifest` | epoch-cheap now |
| Units/slices | `cix-*.service` | `composix-*.service` | read surface; full name |
| Store items | `cix-item-*` | `composix-item-*` | read surface |
| Crates | `cix`, `cix-*` | `composix`, `composix-*` | one rename commit |
| Env vars | `CIX_*` | `COMPOSIX_*` | grep-friendlier |
| Cache/state dirs | `~/.cache/cix` | `~/.cache/composix` | migration = one move |
| Nix interop attr | `buildCixfile` | `buildComposixfile` | follows file |
| Docs vocabulary | mixed "cix"/"composix" | "composix" everywhere; "cix" only if the alias survives | Mathijs's docs redo (0.1) enforces it |

Migration: mechanical, folded into the epoch sweep (the corpus is
being rewritten anyway); crate renames are one commit + CI update;
unit-name change is invisible pre-0.1 (no external deployments).

## 4. Open questions

- **The binary**: full `composix` with a shipped `cix` alias, full
  `composix` with NO alias (maximal unity, kubectl-style typing cost
  accepted), or keep `cix` as the one sanctioned short form (rg-style)
  while everything else goes composix? This is the only cell where
  typing ergonomics and brand unity genuinely pull apart.
- **`Composixfile` vs keeping `Cixfile`**: if the binary keeps a `cix`
  alias, does the file follow the long or the short family?
- Timing: rename inside the epoch sweep (one churn) — confirm.
