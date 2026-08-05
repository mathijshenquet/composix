#!/bin/sh
cd /data
exec /usr/local/bin/docker-entrypoint.sh "$@"
