set -eu

IFS= read -r password <"$CREDENTIALS_DIRECTORY/db-password"
test -n "$password"
PGPASSWORD="$password" "$1" -h /run/postgresql -p 35432 -U mastodon -d postgres -Atqc 'SELECT 1' >/dev/null
printf 'mastodon cleanup fired\n'
