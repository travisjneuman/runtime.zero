#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
target="${1:-$(rustc -vV | awk '/^host:/ {print $2}')}"
output="${2:-$repo/dist}"

case "$target" in
  aarch64-apple-darwin|x86_64-apple-darwin|universal2-apple-darwin|aarch64-pc-windows-msvc|i686-pc-windows-msvc|x86_64-pc-windows-msvc|aarch64-unknown-linux-gnu|x86_64-unknown-linux-gnu) ;;
  *) printf 'unsupported release target: %s\n' "$target" >&2; exit 2 ;;
esac

if [[ -n "$(git -C "$repo" status --porcelain --untracked-files=normal)" ]]; then
  echo 'release packaging requires a clean Git worktree' >&2
  exit 2
fi

version="$(cargo metadata --manifest-path "$repo/Cargo.toml" --locked --no-deps --format-version 1 | python3 -c 'import json,sys; m=json.load(sys.stdin); print(next(p["version"] for p in m["packages"] if p["name"]=="runtime-zero"))')"
commit="$(git -C "$repo" rev-parse HEAD)"
source_date="$(git -C "$repo" show -s --format=%cI HEAD)"
work="$(mktemp -d "${TMPDIR:-/tmp}/rz0-package-metadata.XXXXXXXX")"
trap 'rm -rf "$work"' EXIT INT TERM

binary_name=rz0
if [[ "$target" == universal2-apple-darwin ]]; then
  [[ "$(uname -s)" == Darwin ]] || { echo 'universal macOS packaging requires lipo on macOS' >&2; exit 2; }
  cargo build --manifest-path "$repo/Cargo.toml" --locked --release --bin rz0 --target aarch64-apple-darwin
  cargo build --manifest-path "$repo/Cargo.toml" --locked --release --bin rz0 --target x86_64-apple-darwin
  binary="$repo/target/$target/release/rz0"
  mkdir -p "$(dirname "$binary")"
  pending="$binary.pending-$$"
  trap 'rm -rf "$work"; rm -f "$pending"' EXIT INT TERM
  lipo -create \
    "$repo/target/aarch64-apple-darwin/release/rz0" \
    "$repo/target/x86_64-apple-darwin/release/rz0" \
    -output "$pending"
  chmod 0755 "$pending"
  mv -f "$pending" "$binary"
else
  cargo build --manifest-path "$repo/Cargo.toml" --locked --release --bin rz0 --target "$target"
  [[ "$target" == *-windows-msvc ]] && binary_name=rz0.exe
  binary="$repo/target/$target/release/$binary_name"
fi
python3 "$repo/scripts/generate_release_metadata.py" \
  --repo "$repo" \
  --target "$target" \
  --binary "$binary" \
  --output "$work/evidence" \
  --version "$version" \
  --source-commit "$commit" \
  --source-date "$source_date" \
  >/dev/null
python3 "$repo/scripts/package_release.py" \
  --repo "$repo" \
  --target "$target" \
  --binary "$binary" \
  --output "$output" \
  --version "$version" \
  --source-commit "$commit" \
  --sbom "$work/evidence/SBOM.spdx.json" \
  --notices "$work/evidence/THIRD-PARTY-NOTICES.txt"
