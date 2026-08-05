#!/bin/sh
set -eu

mode=${1:-}

probe() {
    url=$1
    attempt=0
    while [ "$attempt" -lt 20 ]; do
        if body=$(curl --fail --silent --show-error --max-time 3 "$url"); then
            case "$body" in
                *"It works!"*) return 0 ;;
            esac
        fi
        attempt=$((attempt + 1))
        sleep 1
    done
    return 1
}

check_docker() {
    image=cix-httpd-check-$$
    container=cix-httpd-check-$$
    cleanup() {
        docker rm -f "$container" >/dev/null 2>&1 || true
    }
    trap cleanup EXIT HUP INT TERM
    docker build --tag "$image" --file context/Dockerfile context
    docker run --detach --rm --name "$container" --publish 127.0.0.1:18080:80 "$image" >/dev/null
    probe http://127.0.0.1:18080/
}

check_cix() {
    cix=${CIX:-../../../../target/debug/cix}
    build_log=$(mktemp)
    run_log=$(mktemp)
    unit=
    cleanup() {
        if [ -n "$unit" ]; then
            sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true
        fi
        rm -f "$build_log" "$run_log"
    }
    trap cleanup EXIT HUP INT TERM

    "$cix" build .#httpd >"$build_log" 2>&1
    item=$(tail -n 1 "$build_log")
    case "$item" in
        /nix/store/*) ;;
        *) cat "$build_log" >&2; return 1 ;;
    esac

    sudo -n "$cix" run --detach "$item" >"$run_log" 2>&1
    unit=$(tail -n 1 "$run_log")
    [ -n "$unit" ]
    probe http://127.0.0.1/
}

case "$mode" in
    docker) check_docker ;;
    cix) check_cix ;;
    *) echo "usage: $0 docker|cix" >&2; exit 2 ;;
esac
