#!/usr/bin/env bash

set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")" || exit

RED='\033[0;31m'
ORANGE='\033[0;33m'
GREEN='\033[0;32m'
NC='\033[0m'

ERRORS=0
WARNINGS=0

error() {
  printf '%bERROR: %s%b\n' "$RED" "$1" "$NC"
  ERRORS=$((ERRORS + 1))
}

warning() {
  printf '%bWARNING: %s%b\n' "$ORANGE" "$1" "$NC"
  WARNINGS=$((WARNINGS + 1))
}

ok() {
  printf '%bOK: %s%b\n' "$GREEN" "$1" "$NC"
}

run_check() {
  local name="$1"
  shift

  if "$@" >/dev/null 2>&1; then
    ok "$name"
  else
    error "$name"
  fi
}

CARGO_VERSION=$(sed -nE 's/^version = "([^\"]+)"/\1/p' Cargo.toml | head -n 1)
MESON_VERSION=$(sed -nE "s/^[[:space:]]*version: '([^']+)',[[:space:]]*$/\1/p" meson.build | head -n 1)
RELEASE_INFO=$(sed -nE 's/.*<release version="([^\"]+)" date="([^\"]+)".*/\1 \2/p' \
  data/io.github.alescdb.mailviewer.metainfo.xml.in | head -n 1)

METAINFO_VERSION=${RELEASE_INFO%% *}
METAINFO_DATE=${RELEASE_INFO#* }

printf 'versions :\n'
printf '  - Cargo.toml  : %s\n' "${CARGO_VERSION:-missing}"
printf '  - meson.build : %s\n' "${MESON_VERSION:-missing}"
printf '  - metainfo    : %s\n' "${METAINFO_VERSION:-missing}"

# shellcheck disable=SC2055
if [[ -z "$CARGO_VERSION" || -z "$MESON_VERSION" || -z "$METAINFO_VERSION" ]]; then
  error 'Unable to read all versions.'
elif [[ "$CARGO_VERSION" != "$MESON_VERSION" || "$CARGO_VERSION" != "$METAINFO_VERSION" ]]; then
  error 'Cargo.toml, meson.build and metainfo.xml.in contain different versions.'
else
  ok "version: $CARGO_VERSION"
fi

TODAY=$(date +%F)
if [[ -z "$METAINFO_DATE" ]]; then
  error 'Unable to read the latest release date.'
elif [[ "$METAINFO_DATE" != "$TODAY" ]]; then
  warning "latest release date is $METAINFO_DATE, today is $TODAY."
else
  ok "release date: $METAINFO_DATE"
fi

run_check 'tests' cargo test
run_check 'clippy' cargo clippy --all-targets -- -D warnings
run_check 'fmt' cargo +nightly fmt -- --check
run_check 'build' cargo build

if [[ -n "$(git status --porcelain)" ]]; then
  error 'repository contains uncommitted changes or files.'
else
  ok 'repository is clean.'
fi

if [[ -z "${CARGO_VERSION:-}" ]]; then
  error 'git tag cannot be checked without a version.'
else
  HEAD_COMMIT=$(git rev-parse HEAD)
  TAG_COMMIT=$(git rev-parse -q --verify "refs/tags/$CARGO_VERSION^{commit}" 2>/dev/null || true)

  if [[ -z "$TAG_COMMIT" ]]; then
    error "git tag $CARGO_VERSION does not exist."
  elif [[ "$TAG_COMMIT" != "$HEAD_COMMIT" ]]; then
    error "git tag $CARGO_VERSION does not point to the current commit."
  else
    ok "git tag $CARGO_VERSION points to the current commit."
  fi
fi

printf '\n'
if [[ "$ERRORS" -eq 0 ]]; then
  if [[ "$WARNINGS" -gt 0 ]]; then
    printf '%brelease valid with %d warning(s).%b\n' "$ORANGE" "$WARNINGS" "$NC"
  else
    printf '%brelease valid.%b\n' "$GREEN" "$NC"
  fi
  printf '\nInfo :\n  git push --atomic origin main %s\n' "$CARGO_VERSION"
  exit 0
fi

printf '%b%d error(s), %d warning(s)%b\n' "$RED" "$ERRORS" "$WARNINGS" "$NC"
exit 1
