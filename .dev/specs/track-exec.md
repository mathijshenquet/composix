# track/exec — `cix exec` + `cix debug` (D34)

Read AGENTS.md at the repo root first. Authoritative design: docs/design.md **D34** (plus the
D31 addendum for the exec-PATH/`--with` rationale, D13 for `--user` degradation, D29c for the
generator-as-library). This spec adds implementation guidance; where it and D34 disagree, D34
wins.

## `cix debug` (build this first — higher current value)

`cix debug <installable-or-ref>[#service] [--user] [-e K=V]... [-- cmd args...]`

- Resolution identical to `cix run` (same target grammar, one-service-⇒-optional `#service`).
- Build the unit definition through the existing generator exactly as `cix run` would
  (same properties: projection, seccomp, DynamicUser, dirs, env defaults, `-e` overrides),
  then override the entrypoint: interactive shell by default, or the one-shot `cmd`.
- Shell resolution (shared helper, used by both verbs): first `sh` resolved against the
  service's effective PATH (the generated env.PATH default), else `/bin/sh`, else a hard
  error telling the user to pass `-- cmd`. Report which shell was picked.
- Launch: `systemd-run --pty` (interactive) / plain (one-shot), `--collect`, unit name
  `cix-debug-<svc>-<nonce>` in the same slice scheme as run. Foreground; exit code of the
  shell/cmd propagates.
- Do NOT wire listener `.socket` units — a debug shell inherits no fds. If the spec declares
  listeners, print a one-line note that they are not bound in debug.
- `--user`: D13 degraded mode, same property-fallback path as `cix run --user`, loudly
  labeled.
- Print a loud banner line on entry: debug shell, full service sandbox, service identity.

## `cix exec`

`cix exec <unit-or-service> [--root] [-- cmd args...]`

- Target: an exact unit name as shown by `cix ps`, or a bare service name if it matches
  exactly one running `cix-*` unit (ambiguity ⇒ error listing candidates; none running ⇒
  error suggesting `cix debug`).
- Read `MainPID`, `Environment`, and the runtime `UID`/`GID` via `systemctl show` (mind
  systemd's space-separated quoting when parsing `Environment`).
- Join the target's mount/pid/net/ipc/uts namespaces. Prefer direct `setns(2)` via the
  already-used libc FFI over shelling out to nsenter(1) (no util-linux dependency), but
  shelling out is acceptable if the FFI gets hairy — your call, note it in the LOG. After
  joining the pid ns, fork so the child is a member; child does setgid/setgroups/setuid to
  the service identity (default) before exec. `--root` skips the drop.
- Env for the exec'd process: the unit's recorded `Environment` verbatim, plus pass through
  the caller's `TERM`. Command: `-- cmd` or the shared shell fallback (resolved against the
  unit env's PATH).
- No seccomp/caps/cgroup applied to the joined process (D34, deliberate). System mode
  requires root: error clearly and early if euid != 0, suggesting sudo.
- `--user` variant: no namespace join is possible without root; spawn the command with the
  unit's recorded env at the caller's own uid, loudly labeled degraded (D13 spirit).
- Interactivity: cix is a direct child of the terminal — inherit stdio, no pty machinery.

## Where

`crates/cix-run/src/` (suggest `exec.rs` + `debug.rs`, CLI wiring in `crates/cix` following
the existing subcommand pattern). Reuse, don't fork: env/PATH logic and unit-definition
construction must go through the existing spec/generator code paths.

## Verification gate (all required)

1. `cargo build`, `cargo test`, `cargo fmt --all --check` clean.
2. Unit tests: target disambiguation (exact unit / unique service / ambiguous / none),
   Environment parsing incl. quoted values, shell fallback chain, debug entrypoint override
   (golden-style against the generator output like existing unit tests).
3. Live demos, verified by actually running them (root paths need sudo — do run them, the
   environment allows it; record transcripts in the LOG):
   a. `cix debug` on `examples/nginx` (system mode): inside, verify the projection (item
      mounts visible ro, role dirs writable, `id` shows the dynamic user) and that a denied
      operation is actually denied (e.g. touch outside granted dirs fails).
   b. `cix debug --user` on the same: degraded banner, shell works.
   c. Start `examples/nginx` via `cix run --detach`, then `cix exec` into it: `ps` shows the
      nginx processes (pid ns joined), `id` shows the service uid, state-dir write carries
      the service ownership; `cix exec <svc> --root -- id` shows root. Stop the unit after.
4. Tour: add a scenario page (next number) for `cix debug --user` if it can be made
   deterministic under the harness normalizers (one-shot `-- cmd` invocations, not an
   interactive shell); if determinism is not achievable, say so explicitly in the LOG instead
   of shipping a flaky page. Drift check must pass either way.
5. docs/docker.md ledger: flip the `docker debug` row to ✅ citing D34; update the
   "operational verb set is thin" gap line (exec is no longer missing); add the conscious
   refusal of pet-server shell workflows where it fits (D34). Keep ledger style: dispositions
   cite decisions.
6. Commit on branch track/exec. A finished task with no commit is a failed task.

## Log

Keep .dev/specs/track-exec.LOG.md current (append-only, timestamped): decisions taken
(setns vs nsenter, shell resolution details), demo transcripts, surprises. Spec-boundary
frictions you hit go there too — they feed the next design round.

## Correction round 1 (orchestrator re-verification findings, 2026-07-29)

Independent re-verification found one real defect and one honesty problem. Fix both.

1. **Command resolution fails on units with an empty recorded Environment.** The nginx
   example unit records `Environment=` (empty) — `sudo cix exec nginx -- id` and
   `--root -- id` both fail with "not found on the unit's recorded PATH". Fix: the effective
   PATH for resolution (shell AND explicit commands, exec AND debug) is the recorded/generated
   PATH *followed by* the `/usr/bin:/bin` fallback — operator surgery means operator tools
   must be reachable. Keep the clear error when even that fails. Add a unit test for the
   empty-Environment case.
2. **The LOG transcripts overclaim.** The 11:02 entry says `--root -- id` "returned
   uid=0(root)" and describes pid-ns-joined process listings against this same fixture —
   those invocations cannot have run as written (finding 1) and a port-declaring service has
   NO private pid/net namespace to join (systemd default; only mount/ipc/uts exist here, and
   the process view is the host's). Append a correction entry to the LOG stating what was
   actually run vs claimed; do not rewrite history. Align the banner text, docs/docker.md
   wording, and D34-citing prose to the honest form: exec joins the namespaces the unit
   *has*; pid/net are host-shared unless the spec denied network.
3. Re-run the live gate demos (3a–3c) for real, transcripts in the LOG, including
   `sudo cix exec nginx -- id` and `--root -- id` now passing. Full gate, then commit.
