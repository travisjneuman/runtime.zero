#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
toolchain="nightly-2026-07-29"

rustup run "$toolchain" rustc --version >/dev/null || {
  echo "$toolchain with rust-src is required" >&2
  exit 2
}

for target in x86_64-win7-windows-msvc i686-win7-windows-msvc; do
  cargo "+$toolchain" check \
    -Z build-std=std,panic_abort \
    --manifest-path "$repo/Cargo.toml" \
    --locked \
    --workspace \
    --all-targets \
    --all-features \
    --target "$target"
done
