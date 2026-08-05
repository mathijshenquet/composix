#!/usr/bin/env bash

if [[ "${INIT_ASSETS}" == "1" ]] && [[ ! -f "/www/assets/config.yml" ]]; then
    echo "No configuration found, installing default config & assets"
    if [[ -w "/www/assets/" ]]; then
        cp -Rn /www/default-assets/. /www/assets/
        cp -n /www/default-assets/config.yml.dist /www/assets/config.yml
    else
        echo "Assets directory not writable, skipping default config install."
        echo "Check assets directory permissions & docker user or skip default assets install by setting the INIT_ASSETS env var to 0."
    fi
fi

echo "Starting webserver"
exec 3>&1
exec lighttpd -D -f /lighttpd.conf
