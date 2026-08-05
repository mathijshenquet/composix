#!/usr/bin/env bash

set -euo pipefail

mkdir -p /var/www/html/images/uploads/logos/avatars /var/run

php-fpm -F -y /etc/wallos-php-fpm.conf &
php_fpm_pid=$!

nginx -c /etc/wallos-nginx.conf -g 'daemon off;' &
nginx_pid=$!

shutdown() {
    kill -TERM "$nginx_pid" "$php_fpm_pid" 2>/dev/null || true
}

trap shutdown SIGTERM SIGINT SIGQUIT

sleep 1
php /var/www/html/endpoints/cronjobs/createdatabase.php
php /var/www/html/endpoints/db/migrate.php
chmod -R 755 /var/www/html/db/
mkdir -p /var/www/html/images/uploads/logos/avatars
chmod -R 755 /var/www/html/images/uploads/logos
php /var/www/html/endpoints/cronjobs/updatenextpayment.php
php /var/www/html/endpoints/cronjobs/updateexchange.php
php /var/www/html/endpoints/cronjobs/checkforupdates.php

wait
