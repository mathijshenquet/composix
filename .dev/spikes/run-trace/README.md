# D38 run-trace spike

This directory is deliberately separate from the product crates and Cixfile language.
`harness/runtrace` is ecosystem-agnostic: it accepts a command, explicit environment,
offered store paths, optional read-only dependency directories, and optional prior outputs.
It provides the networkless sandbox, tracing, memo check, and output hashing.

Each directory under `examples/` owns all ecosystem knowledge, including its lock,
prefetch procedure, command line, and measurement driver. `prepare.sh` is allowed network
access and models a future lock-derived fixed-output dependency fetch. Sandboxed commands
never have network access.

Run all four preparation scripts, then their `measure.sh` scripts. Compact evidence is
written under `results/`; raw strace logs are intentionally ignored.
