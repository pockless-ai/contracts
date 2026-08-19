#!/usr/bin/env bash
set -euo pipefail

CONTRACTS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$HOME/.foundry/bin:$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Required bootstrap command is missing: $1" >&2
    exit 1
  fi
}

require_command curl
require_command git

case "$(uname -s)" in
  Darwin | Linux) ;;
  *)
    echo "Automated tool setup supports macOS and Linux only." >&2
    exit 1
    ;;
esac

if [[ "$(uname -s)" == "Darwin" ]] && ! xcrun --find clang >/dev/null 2>&1; then
  echo "Install Xcode Command Line Tools first: xcode-select --install" >&2
  exit 1
fi

if ! command -v forge >/dev/null 2>&1; then
  echo "Installing Foundry..."
  curl -L https://foundry.paradigm.xyz | bash
  export PATH="$HOME/.foundry/bin:$PATH"
  "$HOME/.foundry/bin/foundryup"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "Installing Rust and Cargo..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs |
    sh -s -- -y --profile minimal
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

if command -v rustup >/dev/null 2>&1; then
  echo "Installing Rust formatting and lint components..."
  rustup component add rustfmt clippy
fi

if ! command -v solana >/dev/null 2>&1; then
  echo "Installing Anza Solana platform tools..."
  sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
  export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
fi

if ! command -v solana-verify >/dev/null 2>&1; then
  echo "Installing solana-verify (this can take several minutes)..."
  cargo install solana-verify --locked
fi

if [[ ! -f "$CONTRACTS_ROOT/evm/lib/forge-std/src/Test.sol" ]]; then
  echo "Installing forge-std..."
  forge install \
    --root "$CONTRACTS_ROOT/evm" \
    --no-git \
    foundry-rs/forge-std
fi

echo
echo "Installed deployment tools:"
forge --version
cargo --version
solana --version
solana-verify --version

echo
echo "Setup complete. The deployment CLI can discover these standard install locations."
echo "To expose the commands directly in future zsh terminals, add:"
echo '  export PATH="$HOME/.foundry/bin:$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"'
