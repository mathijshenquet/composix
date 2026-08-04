#!/bin/sh
set -eu
mkdir -p /var/lib/filestash/state/config
if [ ! -e /var/lib/filestash/state/config/config.json ]; then
  cp /share/filestash-config.json /var/lib/filestash/state/config/config.json
fi
