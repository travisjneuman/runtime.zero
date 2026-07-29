#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
target="${1:-$(rustc -vV | awk '/^host:/ {print $2}')}"
output="${2:-$repo/dist}"

case "$target" in
  aarch64-apple-darwin|x86_64-apple-darwin|aarch64-pc-windows-msvc|x86_64-pc-windows-msvc|aarch64-unknown-linux-gnu|x86_64-unknown-linux-gnu) ;;
  *) printf 'unsupported release target: %s\n' "$target" >&2; exit 2 ;;
esac

if [[ -n "$(git -C "$repo" status --porcelain --untracked-files=normal)" ]]; then
  echo 'release packaging requires a clean Git worktree' >&2
  exit 2
fi

version="$(cargo metadata --manifest-path "$repo/Cargo.toml" --locked --no-deps --format-version 1 | python3 -c 'import json,sys; m=json.load(sys.stdin); print(next(p["version"] for p in m["packages"] if p["name"]=="runtime-zero"))')"
commit="$(git -C "$repo" rev-parse HEAD)"

cargo build --manifest-path "$repo/Cargo.toml" --locked --release --bin rz0 --target "$target"
binary_name=rz0
[[ "$target" == *-windows-msvc ]] && binary_name=rz0.exe
python3 "$repo/scripts/package_release.py" \
  --repo "$repo" \
  --target "$target" \
  --binary "$repo/target/$target/release/$binary_name" \
  --output "$output" \
  --version "$version" \
  --source-commit "$commit"
