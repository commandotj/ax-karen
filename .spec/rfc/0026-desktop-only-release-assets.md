# RFC 0026: Publish only installable Karen desktop app artifacts

**Status:** Draft

## Summary

Restrict future GitHub Releases to installable Karen desktop app packages for the currently supported release targets: macOS arm64 and Linux x86_64. Publish the macOS DMG and Linux AppImage and Debian package. Do not publish Windows builds, standalone CLI archives, or generic desktop `.tar.gz` bundles. Keep the CLI bundled inside the desktop app as its sidecar.

## Problem

`.github/workflows/release.yml` currently publishes standalone CLI archives for Linux, macOS, and Windows, alongside desktop artifacts for macOS, Linux, and Windows. It also publishes generic desktop `.tar.gz` bundles that duplicate the installable packages. Karen's CLI is a sidecar, not a separately distributed product. Windows release targets have not been tested. The current `v0.12.0` release already contains these assets; changing the workflow cannot retroactively remove them.

The workflow also updates both the Luban Homebrew Formula and Cask, and the Cask currently points to a generic desktop `.tar.gz` rather than the macOS installer. This does not match a desktop-app-only distribution contract.

## Goals

- Publish only the macOS arm64 `.dmg`, Linux x86_64 `.AppImage`, and Linux x86_64 `.deb` installers in future GitHub Releases.
- Remove the standalone `build-cli` job and all CLI archives from future releases and `SHA256SUMS`.
- Remove Windows from the desktop build matrix and release packaging.
- Remove generic desktop `.tar.gz` bundles from release assets and checksums.
- Make release publication fail if any required desktop installer is missing.
- Update the Luban tap automation to update only the desktop Cask, using the published DMG URL and checksum; stop updating the CLI Formula.
- Preserve the CLI as the desktop app sidecar and preserve existing release assets, including `v0.12.0`.

## Non-goals

- Removing `apps/cli`, CLI functionality, or `scripts/build-sidecar.mjs`.
- Removing Windows support from the application source or CI outside the release workflow.
- Deleting or rewriting already published CLI, Windows, or `.tar.gz` assets.
- Changing versioning, tag creation, signing, notarization, release triggers, or the current release run.
- Adding macOS Intel or Linux ARM release targets.

## Design

- In `.github/workflows/release.yml`, delete the `build-cli` job, including CLI packaging and artifact upload.
- Keep `build-desktop` entries for macOS arm64 and Linux x86_64 only. Remove Windows-only packaging and checksum steps.
- Package only Tauri's macOS `.dmg` and Linux `.AppImage` and `.deb` outputs. Do not create a tarball of `target/release/bundle`.
- Fail the corresponding build if any expected installer is absent; do not treat missing installer output as optional.
- Set `publish.needs` to `prepare` and `build-desktop`, and download only desktop installer artifacts. Generate master `SHA256SUMS` from those installers only.
- Keep the Luban Cask update and remove the Formula update. Point the Cask to the published `.dmg` and use its verified checksum.
- Leave sidecar compilation, release preparation, tag validation, Apple signing/notarization, draft publication, and the cask PR trigger unchanged.

## Acceptance

- Future releases contain macOS arm64 `.dmg`, Linux x86_64 `.AppImage`, and Linux x86_64 `.deb` installers only.
- No standalone CLI archive, Windows build, `.msi`, Windows zip, or generic desktop `.tar.gz` appears in future releases.
- Missing required installer output fails the build before release publication.
- `SHA256SUMS` covers only the published desktop installers.
- The automated Luban tap PR updates `Casks/karen.rb` to the DMG URL and checksum, and does not update `Formula/karen.rb`.
- The desktop bundle still contains its CLI sidecar.
- Existing `v0.12.0` assets and release remain unchanged.
