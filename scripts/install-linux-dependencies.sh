#!/usr/bin/env bash
set -euo pipefail

# The hosted runner's Azure mirror can stall before every fallback fetch.
# Keep the signed Ubuntu archives already configured by the runner image.
# https://github.com/actions/runner-images/issues/14594
if [[ -f /etc/apt/apt-mirrors.txt ]]; then
  sudo sed -i '\|^http://azure\.archive\.ubuntu\.com/ubuntu/|d' /etc/apt/apt-mirrors.txt
fi

# Persist for Playwright's subsequent install-deps invocation as well.
sudo tee /etc/apt/apt.conf.d/80masterdata-ci >/dev/null <<'APT'
Acquire::Retries "1";
Acquire::http::Timeout "15";
Acquire::https::Timeout "15";
APT

sudo apt-get update --error-on=any
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential libxdo-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev patchelf
