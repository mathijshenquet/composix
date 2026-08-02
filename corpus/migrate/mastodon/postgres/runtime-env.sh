set -eu

state_dir=/var/lib/postgresql
data_dir="$state_dir/data"
uid=$(id -u)
gid=$(id -g)

printf 'mastodon:x:%s:%s:Mastodon PostgreSQL:%s:/noshell\n' \
  "$uid" "$gid" "$state_dir" >"$state_dir/passwd"
printf 'mastodon:x:%s:\n' "$gid" >"$state_dir/group"
export NSS_WRAPPER_PASSWD="$state_dir/passwd"
export NSS_WRAPPER_GROUP="$state_dir/group"
export LD_PRELOAD=/opt/postgres/libnss_wrapper.so
