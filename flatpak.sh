#!/bin/bash

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
    python -m venv .venv
    .venv/bin/pip install aiohttp toml
  fi

  .venv/bin/python flatpak-cargo-generator.py \
    -o mailviewer-sources.json \
    Cargo.lock
fi

##
## Build flatpak
##
flatpak run org.flatpak.Builder \
  --force-clean \
  --sandbox \
  --user \
  --install \
  --install-deps-from=flathub \
  --ccache \
  --mirror-screenshots-url=https://dl.flathub.org/media/ \
  --repo=repo \
  builddir io.github.alescdb.mailviewer.json && {
  ##
  ## Linter
  ##
  flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo repo && {
    echo -e "${GREEN}Lint Success${NC}"
  } || {
    echo -e "${RED}Lint Failed${NC}"
  }
  RUST_LOG=mailviewer=debug flatpak run io.github.alescdb.mailviewer
}
