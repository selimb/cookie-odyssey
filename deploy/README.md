# Provisioning

### nginx

```
sudo rm /etc/nginx/sites-enabled/default
sudo vim /etc/nginx/sites-available/cookie-odyssey
sudo ln -s /etc/nginx/sites-available/cookie-odyssey /etc/nginx/sites-enabled/cookie-odyssey

# verify
nginx -t

# reload
sudo nginx -s reload
```

### systemd

```
sudo vim /etc/systemd/system/cookie-odyssey-app.service

# reload
sudo systemctl daemon-reload

# start
sudo systemctl start cookie-odyssey-app.service

# logs
sudo journalctl -u cookie-odyssey-app.service
```

### bun

Setup `bun` with `mise`:

```
mise use -g bun@1.4.3
```

### Directory layout

Everything lives under `/home/cookie-odyssey`:

```
releases/<id>/   One directory per build, owned by root.
current          Symlink to the release that is running.
previous         Symlink to the release that ran before it.
data/            Owned by cookie-odyssey.
                 Holds .env and db.sqlite.
                 This is the working directory of the service.
```

<!-- [release-dir] -->

`templates` and `assets` are found relative to the binary, so they are part of a release.
`.env` and the database are found relative to the working directory, so they are part of `data/`.

To provision a new host, create `releases/` and `data/`, and write `data/.env`:

```
B=/home/cookie-odyssey
sudo mkdir $B/releases $B/data
sudo chown cookie-odyssey:cookie-odyssey $B/data
sudo vim $B/data/.env
```

The app runs as the `cookie-odyssey` user, which can't be SSH'd into and doesn't have `sudo` privileges.

# Code Deploy

- Builds happen in GitHub Actions (`build.yml`), on push to main and on manual dispatch.
  Each build uploads a `cookie-odyssey` artifact containing `cookie-odyssey.tar.gz`.
  The tarball includes a `VERSION` file (sha, branch, run number, build date).
- Run `./deploy/deploy.sh [run-id]` locally to download the artifact and `scp` it, along with `deploy-remote.ts`, to the VPS.
- On the VPS, from your home directory (where the files were uploaded), run `sudo "$(mise which bun)" deploy-remote.ts` inside `tmux` so that a dropped SSH connection does not interrupt it.
- If the new release has pending migrations (`cookie-odyssey migrate status`), the script lists them and asks before continuing.
  It then stops the service, backs up the database to `~/backups` (the newest 10 are kept), and runs `cookie-odyssey migrate up`.
- It runs `cookie-odyssey check` (assets, every template compiles, no pending migrations, database, storage) before switching `current` to the new release, then restarts the service and health-checks it.
- If anything fails, `current` is switched back to the previous release, along with the database backup if migrations were involved.
  The service is stopped during the migration, so nothing is lost before the new release starts.
  If the new release then fails its health check, writes made in the few seconds since it started are lost by the restore.
  Migrations are never reverted with `down`; the backup is the way back.
  The first deploy has no previous release to switch back to, so a failure leaves the service stopped.
- To roll back manually, run `sudo "$(mise which bun)" deploy-remote.ts --rollback` on the VPS.
  This switches `current` and `previous`, so running it again undoes the rollback.
  It needs a previous release, so it is not available after the first deploy.
  If the database has newer migrations than that release, restore a backup from `~/backups` first.
