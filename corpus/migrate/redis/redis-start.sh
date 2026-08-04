#!/bin/sh
cd /data
exec /bin/docker-entrypoint.sh "$@"
