set -eu
mkdir -p /var/lib/wallos/db /var/lib/wallos/logos
cd /var/www/html
php endpoints/cronjobs/createdatabase.php
php endpoints/db/migrate.php
php endpoints/cronjobs/updatenextpayment.php
php endpoints/cronjobs/updateexchange.php
php endpoints/cronjobs/checkforupdates.php
