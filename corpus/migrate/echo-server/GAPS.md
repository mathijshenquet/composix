Generated: migrate.md@c43ae9b · terra · 2026-07-30
Status: stale — regenerate with CIP-91

- The two runtime files moved from upstream `/app` to filesystem root without a stated reason. → prompt
- Node is linked into `/bin` and then found through implicit self-import instead of an explicit artifact tool declaration. → language ([CIP-91](../../../cips/accepted/0091-artifact-import.md))
- The historical HTTP receipt consumed a one-off build tree that has not been reproduced in the closed-root audit; rerun it from the recorded revision before upgrading the evidence tier. → evidence
