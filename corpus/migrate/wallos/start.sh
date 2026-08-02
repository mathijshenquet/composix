set -eu
php_pid=
cron_pid=
nginx_pid=
cleanup() {
  test -z "$nginx_pid" || kill -QUIT "$nginx_pid" 2>/dev/null || true
  test -z "$php_pid" || kill -QUIT "$php_pid" 2>/dev/null || true
  test -z "$cron_pid" || kill -TERM "$cron_pid" 2>/dev/null || true
}
trap cleanup EXIT INT TERM QUIT
php-fpm -y /etc/php-fpm.conf -F &
php_pid=$!
supercronic /var/www/html/cronjobs &
cron_pid=$!
nginx -c /etc/nginx.conf -e stderr &
nginx_pid=$!
wait -n "$php_pid" "$cron_pid" "$nginx_pid"
