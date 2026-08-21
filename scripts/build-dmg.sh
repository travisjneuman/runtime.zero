#!/usr/bin/env bash
set -euo pipefail
umask 077

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
target="${1:-$(rustc -vV | awk '/^host:/ {print $2}')}"
output="${2:-$repo/dist}"

case "$target" in
  aarch64-apple-darwin|x86_64-apple-darwin|universal2-apple-darwin) ;;
  *) printf 'unsupported macOS DMG target: %s\n' "$target" >&2; exit 2 ;;
esac
if [[ "$(uname -s)" != Darwin ]]; then
  echo 'DMG packaging requires macOS hdiutil' >&2
  exit 2
fi
if [[ -n "$(git -C "$repo" status --porcelain --untracked-files=normal)" ]]; then
  echo 'release packaging requires a clean Git worktree' >&2
  exit 2
fi
if [[ -L "$output" ]]; then
  echo 'output directory cannot be a symlink' >&2
  exit 2
fi
mkdir -p "$output"
output="$(cd "$output" && pwd -P)"

version="$(cargo metadata --manifest-path "$repo/Cargo.toml" --locked --no-deps --format-version 1 | python3 -c 'import json,sys; m=json.load(sys.stdin); print(next(p["version"] for p in m["packages"] if p["name"]=="runtime-zero"))')"
commit="$(git -C "$repo" rev-parse HEAD)"
name="runtime-zero-$version-$target.dmg"
final="$output/$name"
checksum="$final.sha256"
if [[ -e "$final" || -L "$final" || -e "$checksum" || -L "$checksum" ]]; then
  echo "refusing occupied DMG or checksum: $name" >&2
  exit 2
fi

work="$repo/target/release-dmg-work-$commit"
if [[ -e "$work" || -L "$work" ]]; then
  echo "refusing occupied DMG staging directory: $work" >&2
  exit 2
fi
mkdir -p "$work"
pending="$output/.${name}.pending-$$.dmg"
cleanup() {
  rm -rf -- "$work"
  rm -f "$pending"
}
trap cleanup EXIT INT TERM
mkdir "$work/package"
"$repo/scripts/build-package.sh" "$target" "$work/package" >/dev/null
archive="$work/package/runtime-zero-$version-$target.zip"
archive_checksum="$archive.sha256"
python3 "$repo/scripts/prepare_macos_dmg.py" \
  --archive "$archive" \
  --checksum "$archive_checksum" \
  --staging "$work/content" \
  --target "$target" \
  --version "$version" \
  --source-commit "$commit" \
  >"$work/content-manifest.json"

hdiutil create -quiet \
  -fs HFS+ \
  -format UDZO \
  -imagekey zlib-level=9 \
  -volname "runtime.zero-$version" \
  -srcfolder "$work/content" \
  "$pending"
python3 - "$pending" "$final" "$checksum" <<'PY'
import hashlib
import os
from pathlib import Path
import sys

pending, final, checksum = map(Path, sys.argv[1:])
data = pending.read_bytes()
if not data or len(data) > 128 * 1024 * 1024:
    raise SystemExit("DMG is empty or exceeds the container ceiling")
digest = hashlib.sha256(data).hexdigest()
with pending.open("rb") as handle:
    os.fsync(handle.fileno())
os.link(pending, final)
pending.unlink()
line = f"{digest}  {final.name}\n".encode("ascii")
descriptor = os.open(checksum, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
with os.fdopen(descriptor, "wb") as output:
    output.write(line)
    output.flush()
    os.fsync(output.fileno())
directory = os.open(final.parent, os.O_RDONLY)
try:
    os.fsync(directory)
finally:
    os.close(directory)
os.chmod(final, 0o644)
print(f'{{"dmg":"{final}","dmg_sha256":"{digest}","size_bytes":{len(data)}}}')
PY
