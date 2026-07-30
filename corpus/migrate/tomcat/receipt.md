# Receipt

Verdict: **run-fail** (Cixfile class: dissolves / pkgs-only).

Verbatim cix probe transcript:
```
$ sudo systemctl is-active cix-run-tomcat-18c73367d3790ed61.service
inactive
$ curl --max-time 5 --fail --silent --show-error http://127.0.0.1:8080/
curl: (7) Failed to connect to 127.0.0.1 port 8080 after 0 ms: Could not connect to server
```
Cix store path: `/nix/store/rg22y4i573ys3ji2hn4g7qz4x3224ygj-cix-item-tomcat`.
Docker digest: not produced.
