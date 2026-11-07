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
