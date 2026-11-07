# Main-only CI/CD

`.github/workflows/ci-cd.yml` is the only workflow. It runs on pushes to `main`, without
path filters. Other branches, pull requests and tags do not trigger deployment.

1. macOS Apple Silicon, Windows x64 and Linux runners run formatting, strict Clippy and
   unit/UI tests. The macOS and Windows runners also download/cache verified model weights
   and run real Canary/Parakeet speech tests.
2. macOS builds an ad-hoc-signed app and an unsigned PKG, installs it on the ephemeral
   runner and checks executable startup. Windows bundles the official Microsoft C/C++ runtime DLLs, makes
   an NSIS per-user EXE installer, and tests installation, startup and uninstallation.
   Models and settings are preserved by the Windows uninstaller.
3. Only after **all checks and installer builds pass**, the publishing job computes SHA-256
   checksums and uploads a linked `bolum-installers` OCI artifact to GitHub Packages. It
   pulls the artifact back and verifies its checksums before publishing download links.
4. A new Cargo version creates a numbered GitHub Release once. The `latest-main` prerelease
   and its two fixed installer download links refresh on every passing main push. Only
   this explicitly rolling tag is moved; numbered release tags are never overwritten.
5. The documentation site deploys through GitHub Pages after publishing succeeds.

## Downloads

- [Windows installer](https://github.com/shivrajcode/Bolum/releases/download/latest-main/Bolum-windows-x64-setup.exe)
- [macOS installer](https://github.com/shivrajcode/Bolum/releases/download/latest-main/Bolum-macos-arm64.pkg)
- [Numbered releases](https://github.com/shivrajcode/Bolum/releases)
- [Repository package](https://github.com/shivrajcode/Bolum/pkgs/container/bolum-installers)

GitHub Packages stores registry artifacts rather than standalone EXE/PKG release assets.
The package contains the same installers and checksums; the links above provide ordinary
browser downloads. Registry users can retrieve the files with ORAS:

```sh
oras pull ghcr.io/shivrajcode/bolum-installers:main
```

The package also has immutable `sha-<commit>` tags, plus a version tag when a numbered
release is first created. The `main` tag follows the latest successful main build.
GitHub initially creates packages as private; set the package visibility to Public once
if anonymous access is wanted. The repository is connected using OCI source metadata
and the workflow's repository-scoped token.

## Making the next numbered release

Update the version in `Cargo.toml` and `packaging/Info.plist`, increment the plist build
number, refresh `Cargo.lock`, and add `.github/release-notes/v<version>.md`. Push to main.
Without a version change, the pipeline still publishes the new main installers and package.

The workflow uses GitHub's short-lived token with read-only access for builds and scoped
write permissions for publishing. No personal access token is required. GitHub Pages is
configured to deploy with Actions. Signing/notarization certificates are not configured;
these installers therefore retain the documented unknown-publisher prompts.

## Documentation hostname

Deployment is currently gated by the repository variable `DOCS_SITE_READY`. It remains
off while the requested `bolum.github.io` hostname is unresolved: that address belongs
to the existing GitHub user `bolum`, which the connected account does not control.
Repository documentation links are used in the meantime. Do not set the gate to `true`
until the intended hostname and its publishing repository are configured; merely changing
link text cannot give a project ownership of another account's github.io address.
