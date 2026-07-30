# Receipt

Verdict: **run-fail** (Cixfile class: dissolves / pkgs-only).

Verbatim cix run diagnostic:
```
[systemd-run] .../bin/nginx -g "daemon\\" "off\\;"
```
The Cixfile grammar preserved neither quotes nor backslash escapes as one `-g` argument, so nginx exits and the HTTP probe fails.
Cix store path: `/nix/store/f9r3jz7mxjc5h1d706g58sai08vzxr7a-cix-item-nginx`.
Docker digest: not produced.
