#!/bin/sh
# Fetches the newest server image and restarts the server if it changed.
# Run on the Pi by cron every few minutes (docs/server.md). It only ever
# pulls: nothing from outside connects to the Pi.
cd "$(dirname "$0")" || exit 1
docker compose pull --quiet && docker compose up --detach --no-build --quiet-pull
