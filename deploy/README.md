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

`templates` and `assets` are found relative to the binary, so they are part of a release.
`.env` and the database are found relative to the working directory, so they are part of `data/`.
Set `APP.DATABASE_FILE=db.sqlite` in `data/.env`.

### Cutover from the old layout

The old layout was a single `app` directory holding the release files, `.env`, and `db.sqlite`.
The old binary expects `templates` and `assets` in its working directory, so it cannot run from `data/`.
Leave the service stopped until the first deploy with the new layout.

```
sudo systemctl stop cookie-odyssey-app.service
cd /home/cookie-odyssey

sudo mkdir releases data
sudo sh -c 'mv app/db.sqlite* app/.env data/'
sudo chown -R cookie-odyssey:cookie-odyssey data
sudo vim data/.env  # Set APP.DATABASE_FILE=db.sqlite

sudo mv app releases/initial
sudo chown -R root:root releases
sudo ln -s releases/initial current

sudo cp <the new unit file> /etc/systemd/system/cookie-odyssey-app.service
sudo systemctl daemon-reload
```

Then run the deploy as below.
The script starts the service once the new release passes its checks.

# Code Deploy

- Builds happen in GitHub Actions (`build.yml`), on push to main and on manual dispatch.
  Each build uploads a `cookie-odyssey` artifact containing `cookie-odyssey.tar.gz`.
  The tarball includes a `VERSION` file (sha, branch, run number, build date).
- Run `./deploy/deploy.sh [run-id]` locally to download the artifact and scp it, along with `deploy-remote.sh`, to the VPS.
  It uses the latest successful build on main by default, and needs an authenticated `gh` CLI.
  Artifacts expire after 30 days, so re-run the build workflow if the latest one is older.
- On the VPS, run `./deploy-remote.sh` (prompts for the sudo password).
  It installs the build as a new directory in `releases/`, and keeps the previous release for rollback.
  Only the newest 5 releases are kept.
- If the new release has pending migrations (`cookie-odyssey migrate status`), the script lists them and asks before continuing.
  It then stops the service, backs up the database to `~/backups` (the newest 10 are kept), and runs `cookie-odyssey migrate up`.
- It runs `cookie-odyssey check` (assets, templates, no pending migrations, database, storage) before switching `current` to the new release, then restarts the service and health-checks it.
- If anything fails, `current` is switched back to the previous release, along with the database backup if migrations were involved.
  The service is stopped during the migration, so nothing is lost before the new release starts.
  If the new release then fails its health check, writes made in the few seconds since it started are lost by the restore.
  Migrations are never reverted with `down`; the backup is the way back.
- To roll back manually, run `./deploy-remote.sh --rollback` on the VPS.
  This switches `current` and `previous`, so running it again undoes the rollback.
  If the database has newer migrations than that release, restore a backup from `~/backups` first.
