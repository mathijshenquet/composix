# track/tourfix2 — make chapter 5 observability deterministic

Main CI showed that chapter 5's bare `cix ps | head -n 1` transcript is not
deterministic across consecutive render passes. `cix ps` sizes the whole table
before `head` selects its header, while a just-completed `cix debug` transient
unit can still be visible to systemd's asynchronous collection.

Keep the chapter's Cixfiles canonical and its new-user-guide voice. Track and
synchronously unload every user unit created by each observability receipt
before starting the next one. Replace ambient table-header receipts with
queries that select and assert the tour's own canonical observer service, and
keep `cix logs --explain` scoped to that service.

Gate: run the exact `generated_tour_is_deterministic` test at least three times
consecutively, then the standard agent tier: fmt, examples fmt,
warning-denied all-target clippy, serialized full workspace tests, and explicit
tour regeneration plus zero-drift verification. Commit on `track/tourfix2`.
