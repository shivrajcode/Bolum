#!/usr/bin/env bash
set -euo pipefail
# Every successful main push publishes a commit-addressed package and refreshes
# the latest-main installers. Versioned releases are created once, never moved.
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
tag="v$version"
repo_url="https://github.com/$GITHUB_REPOSITORY"
registry="ghcr.io/${GITHUB_REPOSITORY_OWNER,,}/bolum-installers"
notes=".github/release-notes/$tag.md"
test -f "$notes"
cd release-assets
sha256sum *.exe *.pkg *.zip > SHA256SUMS.txt
printf '%s' "$GH_TOKEN" | oras login ghcr.io --username "$GITHUB_ACTOR" --password-stdin
oras push "$registry:sha-$GITHUB_SHA" \
  --artifact-type application/vnd.bolum.installers.v1 \
  --annotation "org.opencontainers.image.source=$repo_url" \
  --annotation "org.opencontainers.image.revision=$GITHUB_SHA" \
  --annotation "org.opencontainers.image.version=$version" \
  --annotation "org.opencontainers.image.description=Bolum Windows EXE and macOS PKG installers. Direct downloads: $repo_url/releases" \
  *.exe *.pkg *.zip SHA256SUMS.txt
oras tag "$registry:sha-$GITHUB_SHA" main
# Check that the registry serves complete, unchanged installer files.
oras pull "$registry:sha-$GITHUB_SHA" --output ../registry-check
(cd ../registry-check && sha256sum -c SHA256SUMS.txt)
cd ..
if ! gh release view "$tag" --repo "$GITHUB_REPOSITORY" >/dev/null 2>&1; then
  gh release create "$tag" --repo "$GITHUB_REPOSITORY" --target "$GITHUB_SHA" \
    --title "Bolum $version" --notes-file "$notes" release-assets/*
  oras tag "$registry:sha-$GITHUB_SHA" "$tag"
fi
mkdir -p continuous-assets
cp "release-assets/Bolum-$version-windows-x64-setup.exe" continuous-assets/Bolum-windows-x64-setup.exe
cp "release-assets/Bolum-$version-macos-arm64.pkg" continuous-assets/Bolum-macos-arm64.pkg
(cd continuous-assets && sha256sum *.exe *.pkg > SHA256SUMS.txt)
cat "$notes" > continuous-assets/notes.md
printf '\n## Latest main build\n\nBuilt from commit `%s`. These downloads update after every passing push to main. Versioned releases remain unchanged.\n' "$GITHUB_SHA" >> continuous-assets/notes.md
if gh release view latest-main --repo "$GITHUB_REPOSITORY" >/dev/null 2>&1; then
  gh api --method PATCH "repos/$GITHUB_REPOSITORY/git/refs/tags/latest-main" -f sha="$GITHUB_SHA" -F force=true
  gh release upload latest-main --repo "$GITHUB_REPOSITORY" --clobber continuous-assets/*.exe continuous-assets/*.pkg continuous-assets/SHA256SUMS.txt
  gh release edit latest-main --repo "$GITHUB_REPOSITORY" --title 'Latest main build' --notes-file continuous-assets/notes.md --prerelease
else
  gh release create latest-main --repo "$GITHUB_REPOSITORY" --target "$GITHUB_SHA" \
    --title 'Latest main build' --notes-file continuous-assets/notes.md --prerelease \
    continuous-assets/*.exe continuous-assets/*.pkg continuous-assets/SHA256SUMS.txt
fi
printf '\nDownloads: %s/releases\nPackage: %s:main\n' "$repo_url" "$registry"
