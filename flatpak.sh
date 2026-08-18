#!/bin/bash

set -e

RED='\033[0;31m'
ORANGE='\033[0;33m'
GREEN='\033[0;32m'
NC='\033[0m'
##
## Create file "mailviewer-sources.json"
##

if [[ "$1" == "--deps" ]]; then
  echo -e "${ORANGE}Building mailviewer-sources.json${NC}"
  if [[ ! -f flatpak-cargo-generator.py ]]; then
    wget https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/refs/heads/master/cargo/flatpak-cargo-generator.py
  fi

  if [[ ! -d .venv ]]; then
    python3 -m venv .venv
    .venv/bin/pip install aiohttp toml tomlkit
  fi

  .venv/bin/python flatpak-cargo-generator.py \
    -o mailviewer-sources.json \
    Cargo.lock
    
  exit 0
fi

##
## Check Manifest
##
current_path=$(realpath "$(dirname "$0")")

echo "Checking Manifest"
flatpak run \
  --filesystem="$current_path" \
  --command=flatpak-builder-lint \
  org.flatpak.Builder manifest io.github.alescdb.mailviewer.json || 

##
## Build flatpak
##
echo "Building Flatpak"
flatpak run --filesystem="$current_path" org.flatpak.Builder \
  --force-clean \
  --sandbox \
  --user \
  --ccache \
  --install \
  --install-deps-from=flathub \
  --mirror-screenshots-url=https://dl.flathub.org/media/ \
  --repo=repo \
  builddir io.github.alescdb.mailviewer.json

##
## Linter
##
echo "Linting"
flatpak run \
  --filesystem="$current_path" \
  --command=flatpak-builder-lint org.flatpak.Builder repo repo

RUST_LOG=mailviewer=debug flatpak run io.github.alescdb.mailviewer
