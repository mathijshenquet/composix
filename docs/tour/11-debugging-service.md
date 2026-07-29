# Debugging a service

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

`cix debug` builds a fresh transient unit from the same service manifest and sandbox compiler as `cix run`, but replaces the service entrypoint with a shell or one-shot command.

```sh
$ cix debug /nix/store/…-service-fixture --user -- /bin/sh -c 'test -n "$CIX_APP" && echo debug-command-ran'
debug-command-ran
warning: cix debug --user is degraded development mode; it does not provide the full system-manager sandbox or DynamicUser identity
=== cix debug: degraded service sandbox; service=tour-service; identity=caller (--user) ===
warning: user manager rejected sandbox controls; degraded fallback required
warning: retrying with degraded sandbox controls (D13)
```

The system-manager form runs as the service's DynamicUser with the complete projection and hardening profile. This rootless tour uses D13's loudly labeled development fallback; a one-shot command keeps the transcript deterministic, while omitting `-- command` opens an interactive shell.


---

[← Previous](10-composing-services.html) · [Tour index](index.html) · [Next →](12-inspecting.html)
