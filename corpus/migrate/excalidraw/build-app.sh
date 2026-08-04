#!/usr/bin/env bash
set -e

rm -rf node_modules excalidraw-app/build
unset NODE_ENV
yarn install --frozen-lockfile --network-timeout 600000
NODE_ENV=production yarn build:app:docker
for prefix in index CodeMirrorEditor; do
  old_path=$(find excalidraw-app/build/assets -maxdepth 1 -type f -name "$prefix-*.js" -print -quit)
  if [ -n "$old_path" ]; then
    old_name=${old_path##*/}
    new_name="$prefix.js"
    find excalidraw-app/build -type f \( -name '*.css' -o -name '*.html' -o -name '*.js' -o -name '*.json' -o -name '*.map' -o -name '*.txt' -o -name '*.webmanifest' -o -name '*.xml' \) -exec sed -i "s|$old_name|$new_name|g" {} +
    mv "$old_path" "excalidraw-app/build/assets/$new_name"
  fi
  old_path=$(find excalidraw-app/build/assets -maxdepth 1 -type f -name "$prefix-*.js.map" -print -quit)
  if [ -n "$old_path" ]; then
    old_name=${old_path##*/}
    new_name="$prefix.js.map"
    find excalidraw-app/build -type f \( -name '*.css' -o -name '*.html' -o -name '*.js' -o -name '*.json' -o -name '*.map' -o -name '*.txt' -o -name '*.webmanifest' -o -name '*.xml' \) -exec sed -i "s|$old_name|$new_name|g" {} +
    mv "$old_path" "excalidraw-app/build/assets/$new_name"
  fi
done
sed -i -E 's#<lastmod>[^<]*</lastmod>#<lastmod>1970-01-01</lastmod>#g' excalidraw-app/build/sitemap.xml
rm -f excalidraw-app/build/sw.js.map
rm -rf node_modules
