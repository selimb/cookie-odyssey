#!/usr/bin/env bash
#
# To be run on the VPS.
set -eux

sudo tar -xzf cookie-odyssey.tar.gz -C /home/cookie-odyssey/app
sudo systemctl restart cookie-odyssey-app.service
