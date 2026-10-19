#!/usr/bin/env bash
#
# To be run on the VPS, by a user with sudo.
# Uploaded to the VPS (along with the build) by deploy.sh.
#
# Layout under /home/cookie-odyssey:
#   releases/<id>/  One directory per build, read-only to the service.
#   current         Symlink to the release that is running.
#   previous        Symlink to the release that ran before it.
#   data/           State: .env, db.sqlite.
#                   This is the working directory of the service.
#
# Installs ~/cookie-odyssey.tar.gz as a new release, and switches `current`
# to it.
#
# If the new release has pending migrations, the service is stopped, the
# database is backed up to ~/backups, and the migrations are applied.
#
# Anything that fails switches back to the previous release and, if
# migrations were involved, restores the database backup.
# The service is stopped for the whole migration, so restoring the backup
# does not lose any writes.
#
# Usage:
#   ./deploy-remote.sh
#   ./deploy-remote.sh --rollback
#
set -eu

base=/home/cookie-odyssey
releases="$base/releases"
current="$base/current"
previous="$base/previous"
data="$base/data"
db="$data/db.sqlite"
releases_keep=5
backups="$HOME/backups"
backups_keep=10
tarball="$HOME/cookie-odyssey.tar.gz"
service=cookie-odyssey-app.service
health_url=http://127.0.0.1:4444/

# The release whose binary run_app uses.
app_bin=""

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
# the data directory.
run_app() {
  sudo -u cookie-odyssey sh -c "cd '$data' && '$app_bin' $*"
}

# Atomically points the symlink $2 at the directory $1.
link() {
  sudo ln -sfn "$1" "$2.tmp"
  sudo mv -T "$2.tmp" "$2"
}

# Backs up the database to $backups, and keeps only the newest few.
backup_db() {
  old_sha="$(sudo sed -n 's/^sha=//p' "$current/VERSION" 2>/dev/null | cut -c1-8)"
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

# Deletes all but the newest few releases, never the current or previous.
prune_releases() {
  keep_current="$(sudo readlink -f "$current")"
  keep_previous="$(sudo readlink -f "$previous" 2>/dev/null || true)"
  # shellcheck disable=SC2012
  sudo ls -1t "$releases" | tail -n +"$((releases_keep + 1))" | while read -r r; do
    case "$releases/$r" in
      "$keep_current" | "$keep_previous") ;;
      *) sudo rm -rf "${releases:?}/$r" ;;
    esac
  done
}

# Switches back to the release in $prev_release (if there was one), and the
# database if it was backed up.
# Leaves the service running.
abort() {
  echo "$1, switching back to the previous release." >&2
  if [ -n "$prev_release" ]; then
    link "$prev_release" "$current"
  fi
  if [ -n "$backup" ]; then
    sudo systemctl stop "$service"
    restore_db
  fi
  sudo systemctl restart "$service"
  wait_healthy || echo "Previous release is not healthy either!" >&2
  exit 1
}

if ! sudo test -d "$data" || ! sudo test -L "$current"; then
  echo "$base is not set up for releases." >&2
  echo "See the cutover steps in deploy/README.md." >&2
  exit 1
fi

if [ "${1:-}" = "--rollback" ]; then
  if ! sudo test -L "$previous"; then
    echo "No previous release." >&2
    exit 1
  fi
  from="$(sudo readlink -f "$current")"
  to="$(sudo readlink -f "$previous")"
  echo "Rolling back to:"
  sudo cat "$to/VERSION"
  app_bin="$to/cookie-odyssey"
  if ! run_app check; then
    echo "Check failed." >&2
    echo "If the database has newer migrations than this release," >&2
    echo "restore a database backup from $backups first." >&2
    exit 1
  fi
  link "$to" "$current"
  link "$from" "$previous"
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

prev_release="$(sudo readlink -f "$current")"

echo "Currently deployed:"
sudo cat "$prev_release/VERSION" 2>/dev/null || echo "(unknown)"
echo "Deploying:"
cat "$staging/VERSION"

run_number="$(sed -n 's/^run_number=//p' "$staging/VERSION")"
sha="$(sed -n 's/^sha=//p' "$staging/VERSION" | cut -c1-8)"
release="$releases/${run_number}-${sha}"
if [ "$release" = "$prev_release" ]; then
  echo "$release is already deployed." >&2
  exit 1
fi

# Install the release.
# The service only needs to read it, so it is owned by root.
sudo rm -rf "$release"
sudo mkdir -p "$release"
sudo cp -a "$staging/." "$release/"
sudo chown -R root:root "$release"
sudo chmod -R u+rwX,go+rX,go-w "$release"
app_bin="$release/cookie-odyssey"

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
# storage) before switching, so that a bad release is rejected early.
run_app check || abort "Check failed"

link "$release" "$current"
sudo systemctl restart "$service"

if wait_healthy; then
  link "$prev_release" "$previous"
  echo "Deployed."
  rm -f "$tarball"
  prune_releases
else
  sudo journalctl -u "$service" --no-pager -n 30 >&2 || true
  abort "New release is not healthy"
fi
