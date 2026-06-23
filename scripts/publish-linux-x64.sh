#!/usr/bin/env bash
set -euo pipefail

COMMIT="${COMMIT:-dev}"
OUTPUT="${OUTPUT:-artifacts/linux-x64}"
RELEASE_BASE_URL="${RELEASE_BASE_URL:-}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_path="$repo_root/$OUTPUT"
VERSION="${VERSION:-$(perl -nle 'print $1 if /^version = "([^"]+)"$/' "$repo_root/Cargo.toml")}" 

cargo build --release --locked

mkdir -p "$output_path"
install -m755 "$repo_root/target/release/jav" "$output_path/jav"

hash="$(sha256sum "$output_path/jav" | awk '{print $1}')"
url=""
if [[ -n "$RELEASE_BASE_URL" ]]; then
  url="$RELEASE_BASE_URL/jav-linux-x64.tar.gz"
fi

cat > "$output_path/release.json" <<EOF
{
  "schema": 1,
  "version": "$VERSION",
  "commit": "$COMMIT",
  "channel": "stable",
  "assets": [
    {
      "rid": "linux-x64",
      "fileName": "jav-linux-x64.tar.gz",
      "sha256": "$hash",
      "url": "$url"
    }
  ]
}
EOF

echo "Published $output_path/jav"
echo "SHA256 $hash"
