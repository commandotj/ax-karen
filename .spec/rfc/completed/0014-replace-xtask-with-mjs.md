# RFC 0014: Replace Rust xtask runner with root mjs script (child of 0001)

**Status:** Implemented

**Parent:** [0001](0001-karen-system-resource-app.md)

## Summary

Replace the non-production `xtask/` Rust binary crate with a zero-dependency
Node `.mjs` task script (`scripts/cargo-check.mjs`) wired into root
`package.json` (`pnpm lint`, `pnpm test`, `pnpm fmt`, `pnpm ci`), so the Cargo
workspace contains only production crates while preserving identical `rustfmt`,
`clippy -D warnings`, and `cargo test` gates locally and in CI.

## Problem

`xtask/` is custom Rust code that is never shipped to users. Keeping it in the
Cargo workspace adds a non-production crate to `Cargo.toml` and `Cargo.lock`,
requires compiling an extra Rust binary before running lint or test commands,
and splits task orchestration between `scripts/*.mjs` (used for sidecar builds)
and `xtask/`.

## Goals

1. Remove the `xtask/` crate from the repository and `Cargo.toml` workspace
   members, and remove the `xtask` alias from `.cargo/config.toml`.
2. Add `scripts/cargo-check.mjs` supporting the exact same subcommands (`lint`,
   `fmt`, `fmt-fix`, `clippy`, `test`, `ci`) and exit-code semantics as `xtask`.
3. Expose `lint`, `test`, `fmt`, `fmt:fix`, `clippy`, and `ci` scripts in the
   root `package.json`.
4. Update `.github/workflows/ci.yml`, `.github/workflows/release.yml`,
   `README.md`, `AGENTS.md`, and active RFC references to use the new `.mjs` /
   `pnpm` commands.

## Non-goals

1. Weakening or changing any `rustfmt` or `clippy` lint rules in `Cargo.toml`.
2. Changing how production Rust crates (`apps/cli`, `crates/monitor-*`,
   `apps/desktop/src-tauri`) are compiled, tested, or packaged.
3. Adding per-package `scripts/` directories or third-party npm dependencies for
   task execution.

## Design

1. **`scripts/cargo-check.mjs`**: Implement a standalone Node `>=22` ES module
   using `node:child_process` (`spawnSync` with `stdio: "inherit"`) that runs:
   - `fmt`: `cargo fmt --all --check`
   - `fmt-fix`: `cargo fmt --all`
   - `clippy`: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - `lint` (default): `fmt` followed by `clippy`
   - `test`: `cargo test --workspace`
   - `ci`: `lint` followed by `test`
2. **Root `package.json`**: Add `"lint": "node scripts/cargo-check.mjs lint"`,
   `"test": "node scripts/cargo-check.mjs test"`,
   `"fmt": "node scripts/cargo-check.mjs fmt"`,
   `"fmt:fix": "node scripts/cargo-check.mjs fmt-fix"`,
   `"clippy": "node scripts/cargo-check.mjs clippy"`, and
   `"ci": "node scripts/cargo-check.mjs ci"`.
3. **Workspace & CI cleanup**:
   - Delete `xtask/Cargo.toml` and `xtask/src/main.rs`.
   - Remove `"xtask"` from `[workspace].members` in `Cargo.toml` and remove the
     `[alias]` block from `.cargo/config.toml`.
   - Update `.github/workflows/ci.yml`, `.github/workflows/release.yml`,
     `README.md`, and `AGENTS.md` to invoke `node scripts/cargo-check.mjs ...`
     or `pnpm lint` / `pnpm test`.

## Acceptance

1. `xtask/` no longer exists and `Cargo.toml` lists only production crates.
2. `node scripts/cargo-check.mjs lint` (`pnpm lint`) and
   `node scripts/cargo-check.mjs test` (`pnpm test`) pass with zero warnings and
   all workspace tests green.
3. `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `README.md`, and
   `AGENTS.md` contain no stale `cargo xtask` references.
