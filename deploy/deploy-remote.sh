#!/usr/bin/env bash
#
# To be run on the VPS, by a user with sudo.
# Uploaded to the VPS (along with the build) by deploy.sh.
#
# Installs ~/cookie-odyssey.tar.gz into the app directory and restarts
# the service.
# The previous release is kept in app.prev, and is restored automatically
# if the new release does not come up healthy.
#
# Usage:
#   ./deploy-remote.sh
#   ./deploy-remote.sh --rollback
#
set -eu

app=/home/cookie-odyssey/app
prev=/home/cookie-odyssey/app.prev
tarball="$HOME/cookie-odyssey.tar.gz"
service=cookie-odyssey-app.service
health_url=http://127.0.0.1:4444/

# Files that make up a release.
# Everything else in the app directory (db.sqlite, etc.) is state.
release_files="cookie-odyssey VERSION assets templates"

wait_healthy() {
  for _ in $(seq 1 15); do
    if sudo systemctl is-active --quiet "$service" &&
      curl -s -o /dev/null -m 2 "$health_url"; then
      return 0
    fi
    sleep 1
  done
  return 1
}

# Replaces the release files in the app directory with those from $1.
install_release() {
  src="$1"
  for f in $release_files; do
    sudo rm -rf "${app:?}/$f"
    sudo cp -a "$src/$f" "$app/$f"
  done
  sudo chown -R cookie-odyssey:cookie-odyssey "$app"
}

if [ "${1:-}" = "--rollback" ]; then
  if ! sudo test -d "$prev"; then
    echo "No previous release in $prev." >&2
    exit 1
  fi
  echo "Rolling back to:"
  sudo cat "$prev/VERSION"
  install_release "$prev"
  sudo systemctl restart "$service"
  wait_healthy || { echo "Rollback is not healthy." >&2; exit 1; }
  echo "Rolled back."
  exit 0
fi

if [ ! -f "$tarball" ]; then
  echo "$tarball not found. Run deploy.sh first." >&2
  exit 1
fi

staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT
tar -xzf "$tarball" -C "$staging"

echo "Currently deployed:"
sudo cat "$app/VERSION" 2>/dev/null || echo "(unknown)"
echo "Deploying:"
cat "$staging/VERSION"

# Save the current release for rollback.
sudo rm -rf "$prev"
sudo mkdir "$prev"
for f in $release_files; do
  if sudo test -e "$app/$f"; then
    sudo cp -a "$app/$f" "$prev/$f"
  fi
done

install_release "$staging"
sudo systemctl restart "$service"

if wait_healthy; then
  echo "Deployed."
  rm -f "$tarball"
else
  echo "New release is not healthy, rolling back." >&2
  sudo journalctl -u "$service" --no-pager -n 30 >&2 || true
  install_release "$prev"
  sudo systemctl restart "$service"
  wait_healthy || echo "Rollback is also unhealthy!" >&2
  exit 1
fi
