#!/usr/bin/env bash
# Idempotent Cloud Agent bootstrap for the Track project.
# Installs the Jujutsu (jj) VCS that Track integrates with, then builds the
# workspace so the CLI binary and test artifacts are ready.
set -euo pipefail

JJ_VERSION="0.45.1"

install_jj() {
  if command -v jj >/dev/null 2>&1; then
    echo "jj already installed: $(jj --version)"
    return 0
  fi

  local arch tarball url tmp
  arch="$(uname -m)"
  case "$arch" in
    x86_64) tarball="jj-v${JJ_VERSION}-x86_64-unknown-linux-musl.tar.gz" ;;
    aarch64 | arm64) tarball="jj-v${JJ_VERSION}-aarch64-unknown-linux-musl.tar.gz" ;;
    *)
      echo "Unsupported architecture for jj install: $arch" >&2
      return 1
      ;;
  esac

  url="https://github.com/jj-vcs/jj/releases/download/v${JJ_VERSION}/${tarball}"
  tmp="$(mktemp -d)"
  echo "Downloading jj ${JJ_VERSION} from ${url}"
  curl -fsSL -o "${tmp}/jj.tar.gz" "$url"
  tar xzf "${tmp}/jj.tar.gz" -C "$tmp" --no-same-owner --no-same-permissions

  if command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; then
    sudo install -m755 "${tmp}/jj" /usr/local/bin/jj
  else
    mkdir -p "${HOME}/.local/bin"
    install -m755 "${tmp}/jj" "${HOME}/.local/bin/jj"
    echo "Installed jj to ${HOME}/.local/bin (ensure it is on PATH)"
  fi

  rm -rf "$tmp"
  echo "Installed jj: $(jj --version)"
}

install_jj

echo "Building the Track workspace..."
cargo build

echo "Bootstrap complete."
