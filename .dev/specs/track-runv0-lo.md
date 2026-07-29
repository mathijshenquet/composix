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
