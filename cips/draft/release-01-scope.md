# release-01-scope — what 0.1 is, derived from the complete open-ends inventory (v2)

Status: **draft v2** (2026-08-06; v1 was rejected by Mathijs as
under-grounded — "heb je alle open eindjes precies laten nalopen?".
v2 is derived from a full sweep of every ledger: all 30 corpus
GAPS.md files (97 routed bullets), docs/docker.md (59 ⏳/❌ rows),
cips/{draft,deferred}, dispositions.md, open-questions.md, and
design.md's backlog. Target sharpened per Mathijs: SERIOUS DOGFOODING
first, then possibly a TARGETED release to selected people/fora; the
bar is "presentable, with no gaping unaddressed holes".)

## 1. The problem, and the bar

0.1 is not a marketing event: it is the moment selected outsiders may
read the repo and try the tool. Two failure modes to design against:
(a) a first-hour wall that the tool cannot explain (gaping hole), and
(b) claims the receipts don't back (presentability). The corollary:
an honest, *named* limitation is presentable; a mute or unexplained
one is not. And before anyone else touches it, we must be living on
it ourselves — dogfooding is the only credible pre-release test of
(a).

## 2. What the inventory actually says

The 97 GAPS bullets + 59 docker.md rows classify cleanly into four
buckets:

**A. Gaping holes — closed or in flight (each names its closer):**
1. The old language itself → the EPOCH (CIP-110..113, adopted;
   groundwork stage 1 running; single corpus sweep pending).
2. fmt breaks build identity (haproxy exhibit) → CIP-110
   (track/fmtkey-impl running).
3. The pnpm/ecosystem-fetch wall (dozzle, verdaccio, directus,
   homer) → frozenStore route + cacert/offline diagnostics
   (track/pnpm-frozenstore running); adopt pnpm-wall as CIP on its
   receipts.
4. State-role realization defect (filebrowser) → FIXED
   (staterole-bindfix merged 2026-08-06).
5. Tour/CI nondeterminism → CLOSED (3 leaks + teardown race fixed).

**B. Gaping holes — NOT yet slotted (the real 0.1 work list):**
6. **Lock-scale** (filestash: 2.7 GiB module cache exceeds the seal
   bound; it-tools: 1.5M-line lock, aggregation −0.52%): a first-hour
   wall for every Go/Node monorepo. The adopted answer is
   `WITH UNSAFE IGNORE` for ecosystem caches (CIP-111) + CIP-99 —
   0.1 requires these two cases GREEN-or-precisely-walled under it.
7. **Probe-executor-under-hardening class** (mailpit ProtectHome
   readiness failure; wallos 203/EXEC; it-tools LOGDIR denial under
   the PrivatePIDs fallback): three cases where OUR OWN harness
   binary cannot run inside the sandbox it configured. Worst
   possible dogfood look; needs a store-backed probe executor
   completion pass (CIP-109's machinery exists — finish the class).
8. **Upstream EXPECT drift** (redis, memcached, haproxy, mosquitto
   tarball/GPG hashes stale as of 2026-08-06): a fresh clone that
   cannot rebuild its own corpus is not presentable. Needs a
   deliberate freshness pass (translation-level EXPECT updates) plus
   a recorded staleness policy (drift is upstream reality; the
   POLICY is what we present).
9. **Small harness defects**: excalidraw's check.sh probes the wrong
   port (acceptance-harness defect, recorded); it-tools SPA
   deep-route fallback open. Sweep-able in one small track.
10. **Docs + naming**: full documentation redo PERSONALLY BY MATHIJS
    (recorded 2026-08-06 — the voice of 0.1 is his), naming settled
    (companion naming-table draft; lean all-composix), rename inside
    the epoch sweep.

**C. Honest walls — presentable AS-IS by framing (no work, a
framing rule):** dissolved twins following nixpkgs versions;
amd64-only narrowing; mastodon-as-composition-proof; renovate
template-only; docker-socket bridges as desk evidence; degraded
PrivatePIDs reporting (the degradation REPORT is the feature);
directus's upstream metadata gap. Rule: every such row keeps its
arrow + receipt, and the release text points AT the ledger as the
product ("we tell you exactly what does not work").

**D. Deliberately out — already dispositioned in docker.md/design.md
(the ⏳/❌ vocabulary):** publish era, reconciler/engine API,
networking era (named networks, netns, D26/D27), swarm/cluster
wholesale, resource limits (compose era), secrets/configs, scale,
`cix init`/prune sugar, SBOM tooling, k8s waves ≥1 (lean: post-0.1),
TAG (deferred), LET-lists, closed-root phase-2 flip. 0.1 presents
these as the roadmap ledger, not as absences.

## 3. Recommendation

0.1 = **buckets A+B done, C framed, D pointed at**, verified by
dogfooding:

- **Dogfood gate (the release test):** a named roster of real
  services runs under composix on our own hosts for a set soak
  window with green probes — proposal: corpus-browser static serve,
  ntfy, valkey (state + restart), one compose composite — 2 weeks
  green before any outside eyes. Dogfood incidents file as corpus
  gaps like everything else.
- **Targeted audience, in order:** 2–5 named nix-adjacent people
  (direct link), then one forum post (NixOS Discourse) — no HN, no
  broad social. The repo IS the release; a tag `v0.1` + README
  claims scoped to what receipts back.
- The corpus (32 cases, receipts, honest arrows) and docs/docker.md
  are the flagship exhibits; Mathijs's docs pass decides their final
  voice.

## 4. Open questions

- **Dogfood roster + soak window**: which services exactly, how long
  green (proposal above; taste call).
- **Freshness policy** (bucket B8): pin-to-drift grace (re-verify
  EXPECTs at release-candidate time only?) vs continuous.
- **Audience list**: the 2–5 names, and which forum first.
- **k8s wave 1**: in (widens audience) or first post-0.1 item
  (protects focus; orchestrator lean).
- **Version semantics**: tag-only v0.1 with `cixManifest` 0 intact
  (lean), or bump the manifest.
- **Probe-executor class (B7)**: one track now, or fold into the
  epoch sweep since units regenerate anyway?
