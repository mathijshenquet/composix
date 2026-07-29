state_dir=/var/lib/postgresql
data_dir="$state_dir/data"
init_dir="$state_dir/.init"
item_dir="$(dirname "$0")/.."
uid="$("$item_dir/bin/id" -u)"
gid="$("$item_dir/bin/id" -g)"

printf 'cix:x:%s:%s:PostgreSQL:%s:/noshell\n' \
  "$uid" "$gid" "$state_dir" > "$state_dir/passwd"
printf 'cix:x:%s:\n' "$gid" > "$state_dir/group"
export NSS_WRAPPER_PASSWD="$state_dir/passwd"
export NSS_WRAPPER_GROUP="$state_dir/group"
export LD_PRELOAD="$item_dir/lib/libnss_wrapper.so"
