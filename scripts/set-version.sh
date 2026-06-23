#!/usr/bin/env bash
set -euo pipefail

if [[ $# -gt 1 ]]; then
  echo "Usage: $0 [version]" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ $# -eq 0 ]]; then
  year="$(date -u +'%Y')"
  month_day="$(date -u +'%-m%-d')"
  base="${year}.${month_day}"
  build_id=1
  while IFS= read -r tag; do
    suffix="${tag#v${base}.}"
    if [[ "$suffix" =~ ^[0-9]+$ && "$suffix" -ge "$build_id" ]]; then
      build_id="$((suffix + 1))"
    fi
  done < <(git -C "$repo_root" tag --list "v${base}.*")
  version="${base}.${build_id}"
else
  version="$1"
fi

if [[ ! "$version" =~ ^[0-9]+(\.[0-9]+)+$ ]]; then
  echo "Invalid version: $version" >&2
  echo "Expected a numeric dotted version with three semver components, for example: 2026.623.1" >&2
  exit 2
fi

cargo_toml="$repo_root/Cargo.toml"
perl -0pi -e 's/^version = ".*?"$/version = "'"$version"'"/m' "$cargo_toml"

echo "Updated Cargo.toml to $version"
