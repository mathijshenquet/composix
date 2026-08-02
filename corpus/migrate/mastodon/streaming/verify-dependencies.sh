set -eu

IFS= read -r password <"$CREDENTIALS_DIRECTORY/db-password"
test -n "$password"
PGPASSWORD="$password" "$1" -h /run/postgresql -p 35432 -U mastodon -d postgres -Atqc 'SELECT 1' >/dev/null
test "$("$2" -s /run/redis/redis.sock PING)" = PONG
printf 'mastodon streaming dependencies ready\n'
