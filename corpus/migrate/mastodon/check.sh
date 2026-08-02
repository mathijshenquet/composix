#!/usr/bin/env bash
set -euo pipefail

mode=${1:?usage: ./check.sh cix}
if [[ $mode != cix ]]; then
  printf 'usage: ./check.sh cix\n' >&2
  exit 1
fi

root=$(
  cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
  pwd
)
repo_root=$(
  cd -- "$root/../../.."
  pwd
)
cix=${CIX:-"$repo_root/target/debug/cix"}
cix=$(realpath "$cix")
if [[ $cix != /nix/store/* ]]; then
  cix=$(nix store add-path "$cix")
fi
work_dir=$(mktemp -d)
state_dir=$(mktemp -d)
compose_dir="$work_dir/compose"
secret_file="$work_dir/db-password"
composite=corpus-mastodon

root_cix() {
  sudo env CIX_STATE_DIR="$state_dir" PATH="$PATH" "$cix" "$@"
}

cleanup() {
  root_cix down "$composite" --purge --yes >/dev/null 2>&1 || true
  sudo systemctl stop 'cix-corpus-mastodon*' >/dev/null 2>&1 || true
  sudo systemctl reset-failed 'cix-corpus-mastodon*' >/dev/null 2>&1 || true
  sudo systemctl daemon-reload >/dev/null 2>&1 || true
  sudo rm -rf -- "$work_dir" "$state_dir"
}
trap cleanup EXIT

sudo -n true
test -x "$cix"
sudo systemctl stop 'cix-corpus-mastodon*' >/dev/null 2>&1 || true
sudo systemctl reset-failed 'cix-corpus-mastodon*' >/dev/null 2>&1 || true
mkdir -p "$compose_dir"
cp "$root/compose.json" "$compose_dir/compose.json"
printf 'mastodon-corpus-password\n' >"$secret_file"
chmod 0600 "$secret_file"
printf 'MASTODON_DB_PASSWORD_FILE=%s\n' "$secret_file" >"$compose_dir/.env"

for member in postgres redis web sidekiq streaming cleanup; do
  item=$(timeout 1200 "$cix" build --update-lock=pkgs "$root/$member#$member")
  printf '%s item %s\n' "$member" "$item"
  root_cix tag "$item" "corpus-mastodon-$member:checked"
done

systemd-analyze calendar '*-*-* *:*:00/5' >/dev/null
root_cix compose check "$compose_dir/compose.json"
start_epoch=$(date +%s)
root_cix up "$compose_dir/compose.json" --update-lock='*'
elapsed=$(($(date +%s) - start_epoch))
if ((elapsed < 3)); then
  printf 'cix up returned before the declared web readiness delay (%ss)\n' "$elapsed" >&2
  exit 1
fi

test "$(curl --fail --silent http://127.0.0.1:33000/health)" = OK
test "$(curl --fail --silent http://127.0.0.1:34000/api/v1/streaming/health)" = OK
shared=/var/lib/cix-compose/corpus-mastodon/shared/public-system
sudo grep -Fx 'credential-source=CREDENTIALS_DIRECTORY' "$shared/web-started"
sudo grep -Fx 'postgres=ok redis=ok' "$shared/web-ready"
sudo grep -Fx 'postgres=ok redis=ok' "$shared/sidekiq-ready"
test "$(sudo systemctl show cix-corpus-mastodon-web.service -p Type --value)" = exec
sudo systemctl show cix-corpus-mastodon-web.service -p ExecStartPost --value | grep -F 'probe await http :33000/health'
test "$(sudo systemctl show cix-corpus-mastodon-web.service -p WatchdogUSec --value)" = 6s
test "$(sudo systemctl is-active cix-corpus-mastodon-cleanup.timer)" = active
sleep 7
test "$(sudo systemctl is-active cix-corpus-mastodon-web.service)" = active
test "$(sudo systemctl is-active cix-corpus-mastodon-sidekiq.service)" = active

cleanup_log=
for _ in {1..80}; do
  cleanup_log=$(sudo journalctl --no-pager --since "@$start_epoch" CIX_COMPOSITE=corpus-mastodon CIX_SERVICE=cleanup)
  if grep -Fq 'mastodon cleanup fired' <<<"$cleanup_log"; then
    break
  fi
  sleep 0.25
done
grep -F 'mastodon cleanup fired' <<<"$cleanup_log"

web_logs=$(root_cix logs corpus-mastodon/web --since "@$start_epoch" -n 50 2>&1)
printf '%s\n' "$web_logs"
grep -F 'mastodon web ready' <<<"$web_logs"
if grep -Fq 'mastodon sidekiq worker ready' <<<"$web_logs"; then
  printf 'cix logs corpus-mastodon/web leaked sidekiq records\n' >&2
  exit 1
fi

printf 'PASS cix: shared-rw, readiness, credential, timer, and member logs\n'
