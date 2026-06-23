#!/usr/bin/env bash
set -euo pipefail

COMMIT="${COMMIT:-dev}"
OUTPUT="${OUTPUT:-artifacts/win-x64}"
RELEASE_BASE_URL="${RELEASE_BASE_URL:-}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_path="$repo_root/$OUTPUT"
VERSION="${VERSION:-$(perl -nle 'print $1 if /^version = "([^"]+)"$/' "$repo_root/Cargo.toml")}" 

cargo build --release --locked --target x86_64-pc-windows-gnu

mkdir -p "$output_path"
install -m755 "$repo_root/target/x86_64-pc-windows-gnu/release/jav.exe" "$output_path/jav.exe"

hash="$(sha256sum "$output_path/jav.exe" | awk '{print $1}')"
url=""
if [[ -n "$RELEASE_BASE_URL" ]]; then
  url="$RELEASE_BASE_URL/jav-win-x64.zip"
fi

cat > "$output_path/release.json" <<EOF
{
  "schema": 1,
  "version": "$VERSION",
  "commit": "$COMMIT",
  "channel": "stable",
  "assets": [
    {
      "rid": "win-x64",
      "fileName": "jav-win-x64.zip",
      "sha256": "$hash",
      "url": "$url"
    }
  ]
}
EOF

echo "Published $output_path/jav.exe"
echo "SHA256 $hash"
