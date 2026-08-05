#!/bin/sh
set -e

cd /var/www/html
exec /usr/local/bin/entrypoint.sh php -d upload_max_filesize=128M -d post_max_size=128M -d memory_limit=1G -d max_execution_time=600 -d max_input_vars=5000 -S '[::]:8080' -t /var/www/html
