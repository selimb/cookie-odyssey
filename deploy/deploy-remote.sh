#!/usr/bin/env bash
#
# To be run on the VPS, by a user with sudo.
# Uploaded to the VPS (along with the build) by deploy.sh.
#
# Installs ~/cookie-odyssey.tar.gz into the app directory and restarts
# the service.
# The previous release is kept in app.prev.
#
# If the new release has pending migrations, the service is stopped, the
# database is backed up to ~/backups, and the migrations are applied.
#
# Anything that fails restores the previous release and, if migrations
# were involved, the database backup.
# The service is stopped for the whole migration, so restoring the backup
# does not lose any writes.
#
# Usage:
#   ./deploy-remote.sh
#   ./deploy-remote.sh --rollback
#
set -eu

app=/home/cookie-odyssey/app
prev=/home/cookie-odyssey/app.prev
db="$app/db.sqlite"
backups="$HOME/backups"
backups_keep=10
tarball="$HOME/cookie-odyssey.tar.gz"
service=cookie-odyssey-app.service
health_url=http://127.0.0.1:4444/

# Files that make up a release.
# Everything else in the app directory (db.sqlite, etc.) is state.
release_files="cookie-odyssey VERSION assets templates"

# Set once the database has been backed up.
backup=""

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

# Runs `cookie-odyssey $@` the way the service would: as the app user, from
# the app directory.
run_app() {
  sudo -u cookie-odyssey sh -c "cd '$app' && ./cookie-odyssey $*"
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

# Backs up the database to $backups, and keeps only the newest few.
backup_db() {
  old_sha="$(sudo sed -n 's/^sha=//p' "$app/VERSION" 2>/dev/null | cut -c1-8)"
  mkdir -p "$backups"
  backup="$backups/db-$(date -u +%Y%m%dT%H%M%SZ)-${old_sha:-unknown}.sqlite"
  sudo sqlite3 "$db" ".backup '$backup'"
  sudo chown "$(id -un):$(id -gn)" "$backup"
  chmod 600 "$backup"
  echo "Database backed up to $backup"

  # shellcheck disable=SC2012
  ls -1t "$backups"/db-*.sqlite | tail -n +"$((backups_keep + 1))" |
    xargs -r rm -f
}

# Restores the database from $backup.
# The service must be stopped.
restore_db() {
  sudo rm -f "$db-wal" "$db-shm"
  sudo cp "$backup" "$db"
  sudo chown cookie-odyssey:cookie-odyssey "$db"
  echo "Database restored from $backup"
}

# Puts back the previous release, and the database if it was backed up.
# Leaves the service running.
abort() {
  echo "$1, restoring the previous release." >&2
  install_release "$prev"
  if [ -n "$backup" ]; then
    sudo systemctl stop "$service"
    restore_db
  fi
  sudo systemctl restart "$service"
  wait_healthy || echo "Previous release is not healthy either!" >&2
  exit 1
}

if [ "${1:-}" = "--rollback" ]; then
  if ! sudo test -d "$prev"; then
    echo "No previous release in $prev." >&2
    exit 1
  fi
  echo "Rolling back to:"
  sudo cat "$prev/VERSION"
  install_release "$prev"
  if ! run_app check; then
    echo "Check failed." >&2
    echo "If the database has newer migrations than this release," >&2
    echo "restore a database backup from $backups first." >&2
    exit 1
  fi
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

# The new binary is the one that knows about the new migrations.
pending="$(run_app migrate status | grep '^pending ' || true)"
if [ -n "$pending" ]; then
  echo "Pending migrations:"
  echo "$pending"
  read -r -p "Stop the service and apply them? [y/N] " answer
  if [ "$answer" != "y" ]; then
    abort "Declined"
  fi

  sudo systemctl stop "$service"
  backup_db
  run_app migrate up || abort "Migration failed"
fi

# Run the app's own checks (assets, templates, migrations, database,
# storage) before starting, so that a bad release is rejected early.
run_app check || abort "Check failed"

sudo systemctl restart "$service"

if wait_healthy; then
  echo "Deployed."
  rm -f "$tarball"
else
  sudo journalctl -u "$service" --no-pager -n 30 >&2 || true
  abort "New release is not healthy"
fi
