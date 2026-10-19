#!/usr/bin/env bash
#
# Bundles an already-built app into dist/cookie-odyssey.tar.gz.
# Expects `just build` to have been run.
# Used by the GitHub build workflow.
#
# Version info is read from the GitHub Actions environment when available,
# and falls back to local git info otherwise.
#
# Usage:
#   ./deploy/package.sh
#
set -eux

git_sha="${GITHUB_SHA:-$(git rev-parse HEAD)}"
branch="${GITHUB_REF_NAME:-$(git rev-parse --abbrev-ref HEAD)}"
run_number="${GITHUB_RUN_NUMBER:-local}"
built_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

rm -rf dist
mkdir dist

cp target/release/cookie-odyssey dist/
mkdir dist/assets
cp -r assets/dist dist/assets/dist
cp -r templates dist/templates

# [VERSION]
cat > dist/VERSION <<VERSION
sha=${git_sha}
branch=${branch}
run_number=${run_number}
built_at=${built_at}
VERSION

(cd dist && tar -czf cookie-odyssey.tar.gz VERSION cookie-odyssey assets templates)
