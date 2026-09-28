#!/usr/bin/env bash
# Re-stamp the Tauri-built .deb with the fork's release version from
# package.json ("releaseVersion", e.g. 0.9.7.1). Cargo/Tauri only accept
# three-part semver, so the bundle is built as "version" and re-versioned here.
#
# usage: scripts/stamp-deb-version.sh   (run after: bun run tauri build --bundles deb --no-sign)
set -euo pipefail
cd "$(dirname "$0")/.."

base=$(node -p 'require("./package.json").version')
release=$(node -p 'require("./package.json").releaseVersion || require("./package.json").version')
bundle_dir=src-tauri/target/release/bundle/deb
source_deb="$bundle_dir/Handy_${base}_amd64.deb"
target_deb="$bundle_dir/Handy_${release}_amd64.deb"

[[ -f "$source_deb" ]] || { echo "missing $source_deb; build the deb first" >&2; exit 1; }
if [[ "$base" == "$release" ]]; then
  echo "$source_deb already carries version $release"
  exit 0
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
dpkg-deb -R "$source_deb" "$work/pkg"
sed -i "s/^Version: .*/Version: ${release}/" "$work/pkg/DEBIAN/control"
fakeroot dpkg-deb --build -Zgzip "$work/pkg" "$target_deb" >/dev/null
rm -f "$source_deb"

echo "Built $target_deb (Version: $(dpkg-deb -f "$target_deb" Version))"
sha256sum "$target_deb"
