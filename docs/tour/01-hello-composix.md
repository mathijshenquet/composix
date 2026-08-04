# Chapter 1: Hello, composix

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

You will build a small nginx service from a canonical Cixfile and inspect the resulting runtime contract. Afterwards, you will understand the shortest path from checked-in files to a production service and why its live receipt belongs to the system-manager scenario.

Composix is a nix-native Docker analogue. Images become immutable Nix store items, and containers become hardened systemd units. Dockerfiles become Cixfiles that say exactly what enters an item and what its process may use.

## Before you start

You need Nix with flakes enabled, `cix`, and a running systemd user manager for this rootless walkthrough. Production uses the system manager; `--user` is the deliberately degraded development path and says so when you invoke it.

Production `cix run` projects the item and its writable role directories. A user manager may reject sandbox properties that the system manager accepts, so this rootless chapter stops at a host-independent inspection and Chapter 5 completes the runtime story.

## Build the item

Your first Cixfile imports nginx, copies two ordinary project files, names its entrypoint and port, and declares nginx's cache- and runtime-lifetime writable directories.

```sh
$ cat Cixfile index.html nginx.conf
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

SERVICE hello
IMPORT ${pkgs.nginx}
COPY index.html /srv/www/index.html
COPY nginx.conf /etc/nginx/nginx.conf
START nginx -c /etc/nginx/nginx.conf -e stderr -g 'pid /run/nginx/nginx.pid;'
PORT http = 18085
CACHEDIR /var/cache/nginx
RUNDIR /run/nginx
<h1>hello from your first composix service</h1>
daemon off;
error_log stderr info;
events { }
http {
  access_log off;
  client_body_temp_path /var/cache/nginx/client-body;
  server { listen 18085; root /srv/www; }
}
```

```sh
$ cix build .
{"hello":"/nix/store/…-cix-item-hello"}
```

```sh
$ cat /nix/store/…-cix-item-hello/cix-manifest.json
{"cixManifest":0,"dirs":{"cache":["/var/cache/nginx"],"run":["/run/nginx"]},"env":{"PATH":{"default":"bin"}},"mounts":["/bin/nginx","/etc/nginx","/share/man","/srv/www"],"ports":{"http":{"protocol":"tcp","value":18085}},"start":["bin/nginx","-c","/etc/nginx/nginx.conf","-e","stderr","-g","pid /run/nginx/nginx.pid;"]}
```

## Run the production contract

> **Not executed here — system-manager scenario:** run the item with `cix run <item> --detach`, request its HTTP port, then stop the printed unit. The [VM dogfood scenario](https://github.com/mathijshenquet/composix/blob/main/nix/vm-dogfood.nix) executes that lifecycle with the item mounts, cache directory, and runtime directory projected by the production manager.

You have now built an immutable item whose imported command, copied absolute-path configuration, port, and writable directories are asserted directly from its manifest. The next chapters unpack the language and runtime model behind it.


---

[Tour index](index.html) · [Next →](02-cixfile-language.html)
