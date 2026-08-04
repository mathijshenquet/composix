#!/usr/bin/env bash
set -euo pipefail

cd /app
runtime_dir=/dev/shm/wallos-cix-$$
mkdir -p /data/db /data/logos/avatars "$runtime_dir/nginx-client" "$runtime_dir/sessions" "$runtime_dir/nginx-proxy" "$runtime_dir/fastcgi-temp" "$runtime_dir/uwsgi-temp" "$runtime_dir/scgi-temp"

fpm_conf="$runtime_dir/php-fpm.conf"
nginx_conf="$runtime_dir/nginx.conf"
while IFS= read -r line; do
  line=${line//\/dev\/shm\/wallos-cix/$runtime_dir}
  printf '%s\n' "$line" >> "$fpm_conf"
done < /etc/wallos/php-fpm.conf
while IFS= read -r line; do
  line=${line//\/dev\/shm\/wallos-cix/$runtime_dir}
  printf '%s\n' "$line" >> "$nginx_conf"
done < /etc/nginx/nginx.conf
export TMPDIR="$runtime_dir"

shutdown_in_progress=0
PHP_FPM_PID=
NGINX_PID=

shutdown_once() {
  shutdown_in_progress=1
  nginx -c "$nginx_conf" -s quit || true
  [ -n "${PHP_FPM_PID}" ] && kill -QUIT "${PHP_FPM_PID}" 2>/dev/null || true
}

trap shutdown_once SIGTERM SIGINT SIGQUIT

php-fpm -F -y "$fpm_conf" &
PHP_FPM_PID=$!
nginx -c "$nginx_conf" -e stderr -g 'daemon off;' &
NGINX_PID=$!

sleep 1
php -d "session.save_path=$runtime_dir/sessions" /app/endpoints/cronjobs/createdatabase.php
php -d "session.save_path=$runtime_dir/sessions" /app/endpoints/db/migrate.php
mkdir -p /data/logos/avatars
php -d "session.save_path=$runtime_dir/sessions" /app/endpoints/cronjobs/updatenextpayment.php
php -d "session.save_path=$runtime_dir/sessions" /app/endpoints/cronjobs/updateexchange.php
php -d "session.save_path=$runtime_dir/sessions" /app/endpoints/cronjobs/checkforupdates.php

wait
