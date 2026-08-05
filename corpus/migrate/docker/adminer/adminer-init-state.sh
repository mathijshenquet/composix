#!/bin/sh
set -eu

mkdir -p /var/www/html
for source in /usr/share/adminer/* /usr/share/adminer/.[!.]* /usr/share/adminer/..?*; do
	[ -e "$source" ] || [ -L "$source" ] || continue
	target="/var/www/html/${source##*/}"
	if [ ! -e "$target" ]; then
		[ -L "$target" ] && rm "$target"
		cp -aL "$source" "$target"
	fi
done
mkdir -p /var/www/html/plugins-enabled
