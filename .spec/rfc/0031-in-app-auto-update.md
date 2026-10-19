# RFC 0031: In-app auto-update (Umbrella)

**Status:** Approved

**Type:** Umbrella

## Summary

Let the Karen desktop app find and install newer GitHub Releases itself. The app checks for an update, tells the user, and installs only after the user confirms. Updates are signed so the app can verify them.

## Problem

Karen ships as signed macOS arm64 and Linux x86_64 installers (RFC 0026), but an installed copy never learns about a new release and users must reinstall by hand.

## Goals

- Children 0032 and 0033 are Implemented.
- A released version is offered to an older installed version and, once confirmed, installs and restarts.

## Non-goals

- Silent or forced updates.
- Windows (dropped by RFC 0026) and any other release channel.
- Reopening this umbrella for new work; spawn a new child or umbrella instead.

## Children

| RFC | Concern |
|-----|---------|
| [0032](0032-updater-check-and-prompt.md) | Check for updates and prompt to install |
| [0033](0033-signed-updater-artifacts.md) | Publish signed updater artifacts and latest.json |

## Acceptance

Umbrella may close when 0032 and 0033 are Implemented; never reopen for new work.
