#!/bin/sh
set -e

mkdir -p /var/www/html
cp -Rn /opt/adminer-seed/. /var/www/html/
