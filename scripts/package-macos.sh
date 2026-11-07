#!/bin/sh
# Build a Finder-launchable bundle without Node or Tauri tooling.
set -eu
cargo build --release --locked
app="target/bundle/Bolum.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/bolum "$app/Contents/MacOS/bolum"
cp packaging/Info.plist "$app/Contents/Info.plist"
cp packaging/icon.icns "$app/Contents/Resources/icon.icns"
cp LICENSE THIRD_PARTY_NOTICES.txt "$app/Contents/Resources/"
codesign --force --deep --sign - "$app"
ditto -c -k --sequesterRsrc --keepParent "$app" target/bundle/Bolum-macos.zip
printf 'App: %s\n' "$app"
du -sh "$app" target/bundle/Bolum-macos.zip
version=$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' "$app/Contents/Info.plist")
mkdir -p target/dist
pkg="target/dist/Bolum-$version-macos-arm64.pkg"
pkg_root=$(mktemp -d)
trap 'rm -rf "$pkg_root"' EXIT
ditto "$app" "$pkg_root/Bolum.app"
pkgbuild --analyze --root "$pkg_root" target/bundle/components.plist
# Always install in Applications, even if a development copy exists elsewhere.
if /usr/libexec/PlistBuddy -c 'Print :0:BundleIsRelocatable' target/bundle/components.plist >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c 'Set :0:BundleIsRelocatable false' target/bundle/components.plist
else
  /usr/libexec/PlistBuddy -c 'Add :0:BundleIsRelocatable bool false' target/bundle/components.plist
fi
pkgbuild --root "$pkg_root" --component-plist target/bundle/components.plist --install-location /Applications --identifier computer.bolum.app --version "$version" "$pkg"
codesign --verify --deep --strict "$app"
if [ "${CI:-false}" = true ]; then
  sudo installer -pkg "$pkg" -target /
  codesign --verify --deep --strict /Applications/Bolum.app
  set +e
  /Applications/Bolum.app/Contents/MacOS/bolum status 2>target/dist/startup-check.log
  result=$?
  set -e
  test "$result" = 1
  rg -q 'Bolum is not running' target/dist/startup-check.log
  rm target/dist/startup-check.log
fi
