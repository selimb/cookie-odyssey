#!/usr/bin/env bash
#
# To be run on a local dev box.
# Uploads a build, and deploy-remote.sh, to the VPS.
# Does not install the build or restart the service.
# To do so, run ./deploy-remote.sh on the VPS.
#
# By default, uses the artifact of the latest successful `build.yml` run
# on main.
# Pass a run ID to use a specific `build.yml` run instead.
# Requires an authenticated `gh` CLI.
#
# Usage:
#   ./deploy/deploy.sh
#   ./deploy/deploy.sh <run-id>
#
# To list builds, most recent first, with their run IDs:
#   gh run list --workflow build.yml
#
set -eu

run_id="${1:-}"

if [ -z "$run_id" ]; then
  run_id="$(gh run list \
    --workflow build.yml \
    --branch main \
    --status success \
    --limit 1 \
    --json databaseId \
    --jq '.[0].databaseId // empty')"
  if [ -z "$run_id" ]; then
    echo "No successful build found on main." >&2
    exit 1
  fi
fi

set -x

rm -rf dist
mkdir dist
gh run download "$run_id" --name cookie-odyssey --dir dist

# Show what is being deployed.
tar -xzOf dist/cookie-odyssey.tar.gz VERSION

scp dist/cookie-odyssey.tar.gz cookie-odyssey:cookie-odyssey.tar.gz
scp deploy/deploy-remote.sh cookie-odyssey:deploy-remote.sh

set +x
echo "Uploaded. Now run: ssh cookie-odyssey, then ./deploy-remote.sh"
