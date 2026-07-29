# track/runv0-lo — RUN sandbox: best-effort loopback (apparmor-restricted hosts)

CI run 30471913346: on GitHub's Ubuntu runner, apt-installed bwrap may create the user/net
namespace but `bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted` (Ubuntu's
apparmor bwrap profile restricts capabilities inside the ns). Local hosts differ. Read
AGENTS.md; design context D39.

Fix (product, not CI): in the RUN sandbox setup, loopback-up becomes BEST-EFFORT.
- If bringing up lo fails, proceed with lo down — this is strictly STRONGER isolation, so
  it is not a degradation: print NOTHING on the success path (tour output must stay
  host-invariant).
- If the RUN step subsequently FAILS and lo was down, append one hint line to the error
  context: the step ran without loopback (host apparmor restriction) — relevant if the
  command needed 127.0.0.1.
- Find the mechanism: bwrap's own lo-up vs doing it ourselves — whichever allows the
  distinction; document the choice in the LOG. Never weaken any other sandbox property to
  make lo work.
Gate: workspace fmt/clippy/tests green incl. tour drift/determinism; a unit test for the
error-hint formatting; local `cix build` of examples/build/projB still works; commit on
track/runv0-lo. The true acceptance test is CI on the runner after merge — the
orchestrator watches that.
Keep .dev/specs/track-runv0-lo.LOG.md current.

## Redesign (orchestrator, after the bwrap-fatality finding)

Do NOT patch/vendor bubblewrap. The network promise does not require a netns — it requires
that the command cannot create internet sockets. New mechanism, two tiers:

1. Preferred (unchanged): bwrap `--unshare-net` (netns + lo), when the host allows it.
2. Fallback on loopback-fatal/userns-hybrid hosts: run bwrap WITHOUT `--unshare-net` but
   WITH a seccomp filter passed via `--seccomp <fd>`: a small fixed BPF program that makes
   `socket(2)` (and `socketpair` for the same families) fail with EPERM for AF_INET,
   AF_INET6, and AF_PACKET. AF_UNIX/AF_NETLINK stay allowed (local-only). This denies all
   internet networking at least as strongly as a netns for RUN's promise.
   - Detection: probe once per build (tiny bwrap --unshare-net true); loopback-fatal ⇒ use
     tier 2 for all RUN steps of the build.
   - The BPF program is fixed bytes — hand-assemble or use a minimal builder; no libseccomp
     dependency unless already trivial in-tree. Document the exact filter in the LOG and in
     docs/cixfile.md's sandbox section (tier is an implementation detail; the PROMISE is
     identical, so success output prints nothing tier-specific — tour must stay
     host-invariant).
   - On RUN failure under tier 2, append the hint that localhost networking was unavailable.
3. FETCH is unaffected (host network by design).
Gate as before, plus: a unit test asserting the BPF bytes deny AF_INET socket() and permit
AF_UNIX (run the filter via a tiny helper under the same mechanism if feasible; else test
the program construction against known-good bytes and rely on the CI runner as the live
proof, stated explicitly in the LOG).
