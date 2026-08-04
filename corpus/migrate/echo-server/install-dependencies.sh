#!/usr/bin/env bash
set -euo pipefail

install_dir=/tmp/cix-echo-server-npm-install
package_json=${2:-package.json}
package_lock=${3:-package-lock.json}
rm -rf "$install_dir"
mkdir -p "$install_dir"
cp "$package_json" "$package_lock" "$install_dir"/
npm install --prefix "$install_dir"
rm -rf node_modules
cp -a "$install_dir/node_modules" node_modules
