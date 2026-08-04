set -eu
. /opt/postgres/runtime-env.sh

if test ! -s "$data_dir/PG_VERSION"; then
  "$(readlink -f "$(command -v initdb)")" \
    --pgdata="$data_dir" \
    --username=mastodon \
    --pwfile="$CREDENTIALS_DIRECTORY/db-password" \
    --auth-local=scram-sha-256 \
    --auth-host=scram-sha-256 \
    --encoding=UTF8 \
    --no-locale
fi
chmod 0700 "$data_dir"
