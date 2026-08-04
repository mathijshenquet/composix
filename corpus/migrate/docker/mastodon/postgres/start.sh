set -eu
. /opt/postgres/runtime-env.sh

exec "$(readlink -f "$(command -v "$1")")" \
  -D "$data_dir" \
  -p 35432 \
  -h 127.0.0.1 \
  -k /run/postgresql \
  -c unix_socket_permissions=0770
