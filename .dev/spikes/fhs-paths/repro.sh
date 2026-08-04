#!/usr/bin/env bash
set -euo pipefail

repo=$(git rev-parse --show-toplevel)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/cix-fhs-spike.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT

package_expr='let pkgs = import (builtins.getFlake (toString ./.)).inputs.nixpkgs { system = builtins.currentSystem; }; in'
path_of() {
    nix eval --raw --impure --expr "$package_expr ($1).outPath"
}

glibc=$(path_of 'pkgs.glibc.out')
glibc_bin=$(path_of 'pkgs.glibc.bin')
gcc=$(path_of 'pkgs.stdenv.cc')
musl=$(path_of 'pkgs.pkgsMusl.musl.out')
musl_gcc=$(path_of 'pkgs.pkgsMusl.stdenv.cc')
patchelf=$(path_of 'pkgs.patchelf')
nix_bash=$(path_of 'pkgs.bashInteractive')

nix build --no-link --impure --expr \
    "$package_expr [ pkgs.glibc.out pkgs.glibc.bin pkgs.stdenv.cc pkgs.pkgsMusl.musl.out pkgs.pkgsMusl.stdenv.cc pkgs.patchelf pkgs.bashInteractive ]" \
    >/dev/null

cp "$repo/.dev/spikes/fhs-paths/"*.c "$scratch/"

closure_binds() {
    nix-store --query --requisites "$@" | while IFS= read -r path; do
        printf '%s\0%s\0%s\0' --ro-bind "$path" "$path"
    done
}

mapfile -d '' -t build_binds < <(closure_binds "$gcc" "$musl_gcc" "$patchelf" "$nix_bash")
bwrap \
    --die-with-parent --new-session --unshare-user --uid 0 --gid 0 \
    --dir /nix --dir /nix/store \
    "${build_binds[@]}" \
    --bind "$scratch" /work --chdir /work \
    --clearenv --setenv PATH /empty \
    -- "$nix_bash/bin/bash" -eu -c "
        '$gcc/bin/cc' probe.c -o gnu-fhs
        '$patchelf/bin/patchelf' --set-interpreter /lib64/ld-linux-x86-64.so.2 --remove-rpath gnu-fhs
        '$gcc/bin/cc' -fPIC -shared -Wl,-soname,libcix-fhs-probe.so.1 probe-lib.c -o libcix-fhs-probe.so.1
        '$gcc/bin/cc' probe-needed.c -L. -Wl,-l:libcix-fhs-probe.so.1 -o gnu-needed
        '$patchelf/bin/patchelf' --set-interpreter /lib64/ld-linux-x86-64.so.2 --remove-rpath gnu-needed
        '$musl_gcc/bin/cc' probe.c -o musl-fhs
        '$patchelf/bin/patchelf' --set-interpreter /lib/ld-musl-x86_64.so.1 --remove-rpath musl-fhs
    "

test "$("$patchelf/bin/patchelf" --print-interpreter "$scratch/gnu-fhs")" = /lib64/ld-linux-x86-64.so.2
test -z "$("$patchelf/bin/patchelf" --print-rpath "$scratch/gnu-fhs")"
test "$("$patchelf/bin/patchelf" --print-interpreter "$scratch/gnu-needed")" = /lib64/ld-linux-x86-64.so.2
test -z "$("$patchelf/bin/patchelf" --print-rpath "$scratch/gnu-needed")"
test "$("$patchelf/bin/patchelf" --print-interpreter "$scratch/musl-fhs")" = /lib/ld-musl-x86_64.so.1
test -z "$("$patchelf/bin/patchelf" --print-rpath "$scratch/musl-fhs")"

merge_lib() {
    local package=$1 destination=$2 entry target
    mkdir -p "$destination"
    for entry in "$package/lib"/*; do
        target="$destination/${entry##*/}"
        if [[ ! -e "$target" && ! -L "$target" ]]; then
            ln -s "$entry" "$target"
        fi
    done
}

mkdir -p "$scratch/gnu/lib" "$scratch/gnu/etc" "$scratch/musl/lib"
merge_lib "$glibc" "$scratch/gnu/lib"
ln -s /work/libcix-fhs-probe.so.1 "$scratch/gnu/lib/libcix-fhs-probe.so.1"
ln -s /lib/libc.so "$scratch/musl/lib/ld-musl-x86_64.so.1"
merge_lib "$musl" "$scratch/musl/lib"
printf '/lib\n' >"$scratch/gnu/etc/ld.so.conf"

mapfile -d '' -t glibc_binds < <(closure_binds "$glibc" "$glibc_bin")
base_gnu=(
    --die-with-parent --new-session --unshare-user --uid 0 --gid 0
    --dir /nix --dir /nix/store "${glibc_binds[@]}"
    --ro-bind "$scratch" /work
    --ro-bind "$scratch/gnu/lib" /lib
    --dir /lib64 --symlink /lib/ld-linux-x86-64.so.2 /lib64/ld-linux-x86-64.so.2
    --clearenv
)

without_cache=$(bwrap "${base_gnu[@]}" -- /work/gnu-fhs)
test "$without_cache" = fhs-probe-ok
printf 'GNU PT_INTERP alias (glibc defaults): %s\n' "$without_cache"

if bwrap "${base_gnu[@]}" -- /work/gnu-needed >"$scratch/no-cache.out" 2>&1; then
    echo 'GNU loader unexpectedly searched /lib without cache wiring' >&2
    exit 1
fi
grep -Fq 'libcix-fhs-probe.so.1' "$scratch/no-cache.out"
printf 'GNU /lib SONAME without wiring: unresolved (expected)\n'

with_library_path=$(bwrap "${base_gnu[@]}" --setenv LD_LIBRARY_PATH /lib -- /work/gnu-needed)
test "$with_library_path" = fhs-needed-ok
printf 'GNU /lib SONAME with LD_LIBRARY_PATH: %s\n' "$with_library_path"

bwrap \
    --die-with-parent --new-session --unshare-user --uid 0 --gid 0 \
    --dir /nix --dir /nix/store "${glibc_binds[@]}" \
    --ro-bind "$scratch/gnu/lib" /lib \
    --bind "$scratch/gnu/etc" /etc \
    --ro-bind "$scratch" /work \
    --clearenv \
    -- "$glibc_bin/bin/ldconfig" -C /etc/ld.so.cache -f /etc/ld.so.conf -X
test -s "$scratch/gnu/etc/ld.so.cache"
cache_listing=$("$glibc_bin/bin/ldconfig" -p -C "$scratch/gnu/etc/ld.so.cache")
grep -Fq 'libcix-fhs-probe.so.1 (libc6,x86-64) => /lib/libcix-fhs-probe.so.1' <<<"$cache_listing"

if bwrap "${base_gnu[@]}" --ro-bind "$scratch/gnu/etc" /etc -- /work/gnu-needed >"$scratch/cache.out" 2>&1; then
    echo 'GNU loader unexpectedly read the union cache' >&2
    exit 1
fi
grep -Fq 'libcix-fhs-probe.so.1' "$scratch/cache.out"
grep -aFq "$glibc/etc/ld.so.cache" "$glibc/lib/ld-linux-x86-64.so.2"
printf 'GNU generated /etc/ld.so.cache: ignored (loader is pinned to %s/etc/ld.so.cache)\n' "$glibc"

mapfile -d '' -t bash_binds < <(closure_binds "$nix_bash")
library_path_trace=$(bwrap \
    --die-with-parent --new-session --unshare-user --uid 0 --gid 0 \
    --dir /nix --dir /nix/store "${bash_binds[@]}" \
    --ro-bind "$scratch/gnu/lib" /lib \
    --ro-bind "$scratch/gnu/etc" /etc \
    --dir /lib64 --symlink /lib/ld-linux-x86-64.so.2 /lib64/ld-linux-x86-64.so.2 \
    --clearenv --setenv LD_DEBUG libs --setenv LD_LIBRARY_PATH /lib \
    -- "$nix_bash/bin/bash" -c true 2>&1)
grep -Fq 'calling init: /lib/libc.so.6' <<<"$library_path_trace"
printf 'LD_LIBRARY_PATH shadows the Nix RUNPATH: yes (rejected)\n'

mapfile -d '' -t musl_binds < <(closure_binds "$musl")
musl_result=$(bwrap \
    --die-with-parent --new-session --unshare-user --uid 0 --gid 0 \
    --dir /nix --dir /nix/store "${musl_binds[@]}" \
    --ro-bind "$scratch" /work \
    --ro-bind "$scratch/musl/lib" /lib \
    --clearenv \
    -- /work/musl-fhs)
test "$musl_result" = fhs-probe-ok
printf 'musl alias: %s\n' "$musl_result"
printf 'VERDICT: no clean GNU /lib wiring; stop before phase 2\n'
