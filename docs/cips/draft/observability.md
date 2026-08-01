# Observability: logs, exit causes, and stats without a daemon

Status: draft, 2026-08-01. Source: the ledger's "operational verb set is
thin" gap (docs/docker.md — no `cix logs`, no stable selector, no
per-app retention contract, no status/exit-cause view) plus the
systemd.exec read-through that showed the substrate already carries
almost everything.

## 1. The problem

An operator of a composix host lives in two vocabularies: `cix` for
build/up/ps, raw `journalctl`/`systemctl`/`systemd-cgtop` for everything
observational. Concretely missing: one command for "the logs of this
composite/service" (docker's most-used verb), an honest exit-cause view
("why did this service stop?" — soon including CIP-79's "liveness
watchdog missed"), per-composite log retention, and a basic resource
view. The refusals are already decided: journald owns log bytes (no
logging drivers, D-class), no daemon, no parallel log store.

## 2. Prior work

**Docker**: `docker logs` reads per-container json-files (its own
storage layer — the thing we refuse); `docker events`, `stats`
(cgroup-derived), `inspect` exit codes; logging drivers as a
configurable pipeline. **k8s**: `kubectl logs` proxies the runtime's
files; real retention is outsourced (Loki/ELK).

**systemd/journald natively provides** (systemd.exec, v257):

- `LogExtraFields=` — arbitrary *indexed* journal fields stamped on
  every record of a unit, explicitly documented for "cross-unit log
  record matching". A selector vocabulary for free.
- `$INVOCATION_ID` — a 128-bit per-run id, stamped on every journal
  record (`_SYSTEMD_INVOCATION_ID`), correlating one runtime cycle.
- `$SERVICE_RESULT`/`$EXIT_CODE`/`$EXIT_STATUS` — machine-readable exit
  cause including the distinct `watchdog` value; exposed on the unit as
  `Result=`/`ExecMainStatus=` properties; plus the documented exit-code
  table 200–245 (EXIT_NAMESPACE, EXIT_CREDENTIALS,
  EXIT_STATE_DIRECTORY, …) diagnosing *which sandboxing step* killed a
  spawn.
- `LogNamespace=` — a per-namespace `systemd-journald@` instance with
  its *own storage and retention configuration*; the literal "per-app
  retention contract" the ledger says is missing.
- `LogRateLimitIntervalSec=`/`LogRateLimitBurst=`, `LogFilterPatterns=`,
  `LogLevelMax=` — per-unit log policy knobs.
- cgroup accounting readable per unit (`MemoryCurrent`,
  `CPUUsageNSec`, `IPIngress/EgressBytes`, tasks) via `systemctl show`;
  `systemd-cgtop` for the live view.
- `OnFailure=`/`OnSuccess=` + `$MONITOR_*` — native failure hooks, no
  reconciler needed.

## 3. Recommendation

Everything below is a *projection* of journald/systemd state — cix
stores nothing, daemonizes nothing, and every command documents its raw
equivalent (D48e transparency).

1. **Stamp the selector fields.** Every generated unit gets
   `LogExtraFields=CIX_COMPOSITE=<comp> CIX_SERVICE=<svc>` (`cix run`:
   `CIX_RUN=<unit>`). This is the load-bearing move: from that moment
   `journalctl CIX_COMPOSITE=acme -f` works, indexed, today, with zero
   cix code in the read path.
2. **`cix logs <comp>[/<svc>]`** — a thin argv-translation to
   `journalctl` over those fields; `-f`, `--since`, `-n` pass through
   verbatim. The docs print the equivalent journalctl line on first use
   (teach the substrate, don't hide it).
3. **Exit causes in `cix ps`/`cix inspect`**: read `Result=`,
   `ExecMainStatus=`, `InvocationID=` from systemctl show; render
   `watchdog` as "liveness watchdog missed" (CIP-79), map spawn exit
   codes 200–245 to their meaning ("226: namespace setup failed") — the
   table is a gift to `cix debug` (D34) and costs a lookup array.
4. **Per-composite retention, opt-in**: compose-level
   `logNamespace: true` → `LogNamespace=cix-<comp>` on every member;
   retention/size configured in that namespace's journald config.
   Opt-in because it spawns a journald instance per composite and moves
   logs out of default `journalctl` view (`--namespace=` needed) —
   composable, but a real operational shift the operator must choose.
5. **`cix stats`** — one-shot table from `systemctl show` accounting
   properties per member (memory, CPU time, tasks, IO/IP when
   accounting is on); live view stays `systemd-cgtop` (documented, not
   wrapped).
6. **Not now**: `OnFailure=` alerting hooks (alerting era; the native
   mechanism is noted), per-unit `LogRateLimit`/`LogFilterPatterns`
   compose fields (no corpus demand yet), `docker events` analogue
   (journal `_SYSTEMD_UNIT` transitions already carry it).

## 4. Open questions

1. Field set: also stamp `CIX_ITEM=<store path>` (correlate logs to
   artifact versions — powerful with $INVOCATION_ID) at the cost of
   ~60 bytes per record? Proposal: yes on services, it makes
   "which version logged this" answerable forever.
2. `cix logs` default scope: unit lifetime (all invocations) with a
   `--invocation` filter, or current invocation by default? Proposal:
   unit lifetime, like journalctl.
3. Does `cix run` (unary compose, CIP-77) get `--log-namespace` from
   day one or is namespacing compose-only until asked?
