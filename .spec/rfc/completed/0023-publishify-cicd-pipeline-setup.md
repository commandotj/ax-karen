# RFC 0023: Publishify CI/CD Pipeline and Distribution Setup

**Status:** Implemented

## Summary

Adopt the `publishify` contract for repository CI/CD across `commandotj/ax-karen`. This establishes a two-tier branch gate (`develop` for feature PRs, `main` for promotion PRs), resolves Linux CI compilation errors in `crates/monitor-battery`, upgrades the cross-platform release pipeline with separate CLI and desktop app matrices, introduces fail-closed Apple Developer ID signing and notarization under a protected `release` GitHub Environment, produces SHA256 checksums, and automates packaging pull requests against `luban-homebrew` (branch `develop`).

## Problem

1. **Linux CI Failure:** `crates/monitor-battery/src/lib.rs` imports `std::process::Command` unconditionally, which is only used on macOS (`ioreg`). Under `-D warnings`, `cargo clippy` fails on Linux runners with `error: unused import: std::process::Command`.
2. **Missing Promotion Branch Gate:** GitHub's base-branch filter cannot restrict source branches. Unchecked PRs can target `main` directly from arbitrary branches or external forks without version validation or release policy checks.
3. **Invalid & Deprecated Runners in Release Workflow:** `.github/workflows/release.yml` references non-existent runners `macos-15-intel` and `windows-11-arm`. Standard macOS x86_64 binaries should instead be cross-compiled or built using verified runners.
4. **Unscoped Release Secrets:** Release jobs do not declare `environment: release`, leaving sensitive Apple credentials and notarization keys unmanaged and lacking preflight validation.
5. **Missing Package-Manager Automation:** No automated pull request flow exists for the `luban-homebrew` tap (branch `develop`), leaving formula and cask updates manual and error-prone.

## Goals

1. **Linux Build Fix:** Scope `Command` in `crates/monitor-battery/src/lib.rs` to macOS (`#[cfg(target_os = "macos")]`) so cross-platform CI passes cleanly.
2. **PR Verification & Promotion Guard:**
   - Feature PRs targeting `develop` run full lint, unit test, coverage gates, and desktop bundle smoke tests.
   - Promotion PRs targeting `main` enforce that `head.ref == 'develop'` and `head.repo.full_name == 'commandotj/ax-karen'`, followed by synchronized version verification across `Cargo.toml`, `Cargo.lock`, `package.json`, and `apps/desktop/src-tauri/tauri.conf.json`.
3. **Robust Release Workflow:**
   - Runs exclusively on tags (`v*`) or authorized workflow dispatch.
   - Operates under a protected GitHub Environment `release`.
   - Distributes separate CLI binary archives (`karen-<version>-<arch>-<os>.tar.gz` / `.zip`) and GUI desktop packages (`karen-app-<version>-<arch>-<os>.tar.gz` / `.dmg` / `.msi`).
   - Implements a fail-closed Apple credentials preflight step that validates Developer ID and notarization inputs without revealing secret values.
   - Signs and notarizes macOS artifacts via `xcrun notarytool` and staples tickets.
   - Generates and uploads unified SHA256SUMS.
4. **Homebrew Tap Integration:**
   - Adds an automated PR step against `luban-homebrew` targeting branch `develop` after release assets and checksums are verified.
5. **Setup Guides & Rulesets:**
   - Document the exact Apple Developer credential generation and GitHub Environment setup.
   - Provide the complete GitHub Ruleset JSON configuration for `main` and `develop`.

## Non-goals

1. Direct pushes to `main`, `develop`, or the `luban-homebrew` tap default branch.
2. Modifying monitor sampling algorithms or desktop UI components outside the battery import fix.

## Design

### 1. Battery Import Fix (`crates/monitor-battery/src/lib.rs`)
Wrap `use std::process::Command;` with `#[cfg(target_os = "macos")]` so Linux and Windows builds do not flag unused imports under `-D warnings`.

### 2. CI Workflow (`.github/workflows/ci.yml`)
- Trigger on `pull_request` (targeting `develop` or `main`) and `push` (to `develop` and `main`).
- Add a dedicated `promotion-guard` job that executes when `github.base_ref == 'main'`:
  - Validates `github.event.pull_request.head.ref == 'develop'`.
  - Validates `github.event.pull_request.head.repo.full_name == 'commandotj/ax-karen'`.
  - Runs version consistency script ensuring root `Cargo.toml`, `Cargo.lock`, `apps/desktop/package.json`, and `apps/desktop/src-tauri/tauri.conf.json` share the identical semantic version.
- Preserve existing `check` (Linux, macOS, Windows) and `coverage` (macOS gate) jobs.

### 3. Release Workflow (`.github/workflows/release.yml`)
- Trigger on `push: tags: ['v*']` or `workflow_dispatch`.
- Scope release build jobs to `environment: release`.
- Matrix:
  - Linux CLI & Desktop: `ubuntu-24.04` (x86_64) and `ubuntu-24.04-arm` (aarch64).
  - macOS CLI & Desktop: `macos-15` (arm64 native and x86_64 cross-compile).
  - Windows CLI & Desktop: `windows-2025` (x86_64).
- Fail-closed preflight in macOS job:
  - Asserts presence of `APPLE_CERTIFICATE_BASE64`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_TEAM_ID`, `APPLE_API_KEY`, `APPLE_API_KEY_ID`, `APPLE_API_ISSUER`.
  - Unlocks temporary keychain, validates Developer ID identity, signs `.app`, packages `.dmg`, submits to `notarytool`, staples, and validates Gatekeeper assessment.
- Publishes release notes and assets, then opens a release PR to `luban-homebrew` (branch `develop`).

## Acceptance

1. `cargo clippy --workspace --all-targets --all-features -- -D warnings` succeeds on both macOS and Linux.
2. All workspace tests, lints, and coverage suites pass (`pnpm lint`, `pnpm test`, `pnpm coverage`).
3. `.github/workflows/ci.yml` and `.github/workflows/release.yml` pass static YAML and action schema validation.
4. RFC validated with `specify validate .spec/rfc/0023-publishify-cicd-pipeline-setup.md` and `specify sync-check`.
