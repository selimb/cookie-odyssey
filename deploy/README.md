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

# Code Deploy

- Builds happen in GitHub Actions (`build.yml`), on push to main and on manual dispatch.
  Each build uploads a `cookie-odyssey` artifact containing `cookie-odyssey.tar.gz`.
  The tarball includes a `VERSION` file (sha, branch, run number, build date).
- Run `./deploy/deploy.sh [run-id]` locally to download the artifact and scp it, along with `deploy-remote.sh`, to the VPS.
  It uses the latest successful build on main by default, and needs an authenticated `gh` CLI.
  Artifacts expire after 30 days, so re-run the build workflow if the latest one is older.
- On the VPS, run `./deploy-remote.sh` (prompts for the sudo password).
  It keeps the previous release in `/home/cookie-odyssey/app.prev`, restarts the service, and health-checks it.
  It rolls back automatically if the new release is unhealthy.
- To roll back manually, run `./deploy-remote.sh --rollback` on the VPS.
