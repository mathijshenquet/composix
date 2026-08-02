# Renovate schedule regrade receipt

Regrade: 2026-08-02. This is a deliberately narrow conversion of the wild
Kubernetes CronJob row: it uses the real Nix-packaged Renovate executable and
proves the landed APP/timer and journald-selector mechanisms. It is not a full
Renovate deployment: repository configuration and token delivery remain
unverified, and native secret delivery is not part of this feature wave.

## Build and schedule

```text
devenv shell -- cargo build -p cix
target/debug/cix build -t regrade corpus/regrade/renovate
/nix/store/zvkknma4m5xipnahpc0a4vsmcxjkcssn-cix-item-renovate
sudo env PATH="$PATH" target/debug/cix tag <item> renovate:regrade
systemd-analyze calendar daily
Normalized form: *-*-* 00:00:00
target/debug/cix compose check corpus/regrade/renovate/compose.json
compose corpus-renovate: 1 services, 0 edges, valid
sudo env PATH="$PATH" target/debug/cix up \
  corpus/regrade/renovate/compose.json --update='*'
activated corpus-renovate from \
  /nix/store/gcr7gdk2l2w0pqryxgx4i3krdb2d5nlf-cix-compose-corpus-renovate-generation
systemctl is-active cix-corpus-renovate-renovate.timer
active
```

The generated timer contained the expected native projection:

```ini
[Timer]
OnCalendar=daily
Unit=cix-corpus-renovate-renovate.service
Persistent=true
```

The first manual oneshot run trapped in Node/V8 under W^X. Adding the explicit
`CLAIM jit`, retagging the new item, and updating the compose lock corrected the
conversion. The final run was green:

```text
sudo systemctl start cix-corpus-renovate-renovate.service
Result=success
ExecMainCode=1
ExecMainStatus=0
target/debug/cix logs corpus-renovate/renovate \
  --invocation 2da1f08172084cf08056187f5a31784b -n 20
43.214.1
```

`cix logs` printed the equivalent indexed query with
`CIX_COMPOSITE=corpus-renovate`, `CIX_SERVICE=renovate`, and the invocation ID.
Cleanup used `cix down corpus-renovate` and removed both temporary user/root
`renovate:regrade` tags.
