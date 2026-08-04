#!/usr/bin/env bash
set -eu

mkdir -p /etc/phpmyadmin/ssl /etc/phpmyadmin/conf.d /sessions
cp /share/phpmyadmin-config.inc.php /etc/phpmyadmin/config.inc.php
cp /share/phpmyadmin-helpers.php /etc/phpmyadmin/helpers.php
if [ ! -f /etc/phpmyadmin/config.secret.inc.php ]; then
    secret=$(tr -dc 'a-zA-Z0-9~!@#$%^&*_()+}{?></";.,[]=-' < /dev/urandom | head -c 32)
    printf "<?php\n\$cfg['blowfish_secret'] = '%s';\n" "$secret" > /etc/phpmyadmin/config.secret.inc.php
fi
if [ ! -f /etc/phpmyadmin/config.user.inc.php ]; then
    : > /etc/phpmyadmin/config.user.inc.php
fi
cat > /etc/phpmyadmin/phpmyadmin.ini <<EOF
opcache.memory_consumption=128
opcache.interned_strings_buffer=8
opcache.max_accelerated_files=4000
opcache.revalidate_freq=2
opcache.fast_shutdown=1
session.cookie_httponly=1
session.use_strict_mode=1
allow_url_fopen=Off
max_execution_time=${MAX_EXECUTION_TIME}
max_input_vars=10000
memory_limit=${MEMORY_LIMIT}
post_max_size=${UPLOAD_LIMIT}
upload_max_filesize=${UPLOAD_LIMIT}
date.timezone=${TZ}
session.save_path=${SESSION_SAVE_PATH}
EOF
export PHP_INI_SCAN_DIR=/etc/phpmyadmin
