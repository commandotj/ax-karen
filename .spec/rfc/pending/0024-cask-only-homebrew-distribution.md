# RFC 0024: Cask-only Homebrew distribution

**Status:** Pending

## Summary

Karen is a desktop app product. The standalone `karen` CLI is the desktop
app's bundled sidecar, not a separately distributed product. Stop publishing
CLI archives from the release pipeline and distribute the desktop app through
a Homebrew cask only, amending the two-product contract of
[RFC 0023](../completed/0023-publishify-cicd-pipeline-setup.md).

## Problem

[RFC 0023](../completed/0023-publishify-cicd-pipeline-setup.md) declared separate
CLI and GUI deliverables, so `release.yml` builds five CLI archives per tag
(Linux x86_64/arm64, macOS arm64/Intel, Windows x86_64) and the Homebrew tap
automation maintains both a formula and a cask. The CLI binary only exists as
the desktop app's sidecar (`scripts/build-sidecar.mjs`); there is no
CLI-only install path or audience. The extra matrix job produces artifacts
nobody installs, doubles the tap surface the release job must keep correct,
and the formula's checksum contract was written before the cask existed.

## Goals

* Remove the `build-cli` job and its artifacts from `release.yml`.
* Reduce the `publish` job to desktop artifacts; the master `SHA256SUMS`
  covers desktop assets only.
* Reduce the `homebrew-tap-pr` job to the cask update
  (`Casks/karen.rb`); drop the `Formula/karen.rb` block.
* Seed `systembugtj/luban-homebrew` (branch `develop`) with `Casks/karen.rb`
  built from verified release asset URLs and sha256 checksums.

## Non-goals

* Removing `apps/cli` or the monitor crates; the CLI remains the sidecar and
  development tool.
* Changing the sidecar build or bundling mechanism.
* Changing the release trigger (tags `v*` or dispatch) or signing and
  notarization behavior.
* Existing `v0.11.0` CLI assets remain published; they are simply no
  longer produced for future tags.

## Design

* `release.yml`: delete the `build-cli` job; update `publish` to
  `needs: [prepare, build-desktop]` and drop the `cli-*` download step.
  The `SHA256SUMS` step already iterates `dist/*`, so it covers the desktop
  assets unchanged.
* `release.yml` `homebrew-tap-pr`: keep only the `Casks/karen.rb` update
  block; remove the `Formula/karen.rb` block. The no-changes-staged guard
  still applies.
* `scripts/build-sidecar.mjs`, the desktop build, and `check-versions.mjs`
  are untouched — the sidecar still compiles the CLI for every desktop
  bundle.
* The v0.11.0 release keeps its published CLI assets for anyone who already
  fetched them; the tap seeds the cask only.

## Acceptance

* A new `v*` tag produces desktop artifacts and SHA256SUMS with no CLI
  assets, and the release publishes successfully.
* The automated tap PR updates `Casks/karen.rb` version, URL, and sha256
  against branch `develop`.
* `brew install --cask karen` from the tap installs the desktop app and the
  app launches; a dry-run of the removed CLI matrix confirms nothing
  references it.
