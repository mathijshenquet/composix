# Chapter 1: Hello, composix

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

You will build and run a small nginx service from a Cixfile. Afterwards, you will understand the shortest path from checked-in files to a supervised process.

Composix is a nix-native Docker analogue. Images become immutable Nix store items, and containers become hardened systemd units. Dockerfiles become Cixfiles that say exactly what enters an item and what its process may use.

## Before you start

You need Nix with flakes enabled, `cix`, a running systemd user manager for this rootless walkthrough, and `curl`. Production uses the system manager; `--user` is the deliberately degraded development path and says so when you invoke it.

Because a restricted user manager cannot project item mounts on this host, the development probe reads its checked-in page through the locked source path and uses private `/tmp` files. The same item still declares nginx's production cache and runtime paths; Chapter 5 returns to that runtime boundary explicitly.

## Build the item

Your first Cixfile imports nginx, copies two ordinary project files, names its entrypoint and port, and declares nginx's cache- and runtime-lifetime writable directories.

```sh
$ cat Cixfile index.html nginx.conf
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

SERVICE hello
IMPORT ${pkgs.nginx}
COPY index.html /srv/www/index.html
COPY nginx.conf /etc/nginx/nginx.conf
START nginx -p ${src}/ -c nginx.conf -e stderr
PORT http = 18085
CACHEDIR /var/cache/nginx
RUNDIR /run/nginx
<h1>hello from your first composix service</h1>
daemon off;
pid /tmp/cix-tour-nginx.pid;
error_log stderr info;
events { }
http {
  access_log off;
  client_body_temp_path /tmp/cix-tour-nginx-body;
  server { listen 18085; root .; }
}
```

```sh
$ cix build .
{"hello":"/nix/store/…-cix-item-hello"}
```

```sh
$ cat /nix/store/…-cix-item-hello/cix-manifest.json
{"cixManifest":0,"dirs":{"cache":["/var/cache/nginx"],"run":["/run/nginx"]},"env":{"PATH":{"default":"bin"}},"mounts":["/bin/nginx","/etc/nginx","/share/man","/srv/www"],"ports":{"http":{"protocol":"tcp","value":18085}},"start":["bin/nginx","-p","/nix/store/…-cix-source/","-c","nginx.conf","-e","stderr"]}
```

## Run, probe, stop

```sh
$ cix run /nix/store/…-cix-item-hello --user --detach
cix-run-hello-NONCE.service
warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path
```

```sh
$ curl -fsS http://127.0.0.1:8420
<h1>hello from your first composix service</h1>
```

```sh
$ systemctl --user stop cix-run-hello-NONCE.service
```

You have now built an immutable item, run its declared service, reached it on its declared port, and stopped the transient unit. The next chapters unpack the language and operational model behind those five minutes.


---

[Tour index](index.html) · [Next →](02-distribution.html)
