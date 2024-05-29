#!/usr/bin/env bash
#
# To be run on a local dev box.
#
# Usage:
#   ./deploy/deploy.sh
#
set -eux

rm -rf dist
mkdir dist

cargo build --release
make tw

cp target/release/cookie-odyssey dist/
mkdir dist/assets
cp -r assets/dist dist/assets/dist
cp -r templates dist/templates

(cd dist && tar -czf cookie-odyssey.tar.gz *)

scp dist/cookie-odyssey.tar.gz cookie-odyssey:cookie-odyssey.tar.gz
# ssh cookie-odyssey
