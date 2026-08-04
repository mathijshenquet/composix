#!/bin/sh
set -e

cd /var/www/html
if [ "$ADMINER_DESIGN" = __cix_unset__ ]; then unset ADMINER_DESIGN; fi
if [ "$ADMINER_PLUGINS" = __cix_unset__ ]; then unset ADMINER_PLUGINS; fi
exec /usr/local/bin/entrypoint.sh php -d upload_max_filesize=128M -d post_max_size=128M -d memory_limit=1G -d max_execution_time=600 -d max_input_vars=5000 -S '[::]:8080' -t /var/www/html
