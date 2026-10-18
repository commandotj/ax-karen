# RFC 0013: Redesign application icon with Logo Designer skill (child of 0001)

**Status:** Implemented

**Parent:** [0001](0001-karen-system-resource-app.md)

## Summary

Redesign Karen's application icon, top-bar tray icon and in-app brand emblem
using the `logo-designer` skill workflow (`logos/concepts/`, `logos/iterations/`,
`logos/preview.html`, `logos/export/`), then integrate the exported SVG and PNG
assets into `apps/desktop/src-tauri/icons/`, `assets/`, `README.md` and the
desktop window chrome.

## Problem

Karen's current bundle icon (`apps/desktop/src-tauri/icons/icon.png`) and window
header emblem do not yet reflect a unified, purpose-built steampunk instrument
mark produced and size-verified through the `logo-designer` skill pipeline. The
app needs a distinctive icon silhouette (brass on dark enamel) that reads
unmistakably at 1024px–2048px app-bundle sizes as well as 16px–32px favicon and
top-bar tray sizes.

## Goals

1. Execute the `logo-designer` skill workflow to explore, refine and export a
   distinctive steampunk instrument icon for Karen (`viewBox="0 0 512 512"`,
   self-contained SVG, dark enamel `#2A2A2E` / `#14110E`, brass `#C08A3E` /
   `#D8A657` and crisp off-white engraving `#E8E8EA`).
2. Verify small-size legibility at `64px`, `32px` and `16px` via the
   `logo-designer` favicon size-check strip in `logos/preview.html`, ensuring
   strokes (`stroke-width >= 6`) and negative space survive downscaling.
3. Export the full standard PNG matrix (`16`, `32`, `48`, `192`, `512`, `1024`,
   `2048`) via the `logo-designer` export script (`scripts/export.sh`).
4. Replace the application bundle icons in `apps/desktop/src-tauri/icons/`
   (including `icon.png`, multi-size PNGs, macOS `.icns`, Windows `.ico` and
   the top-bar tray icon asset for RFC 0012), publish the master SVG/PNG under
   `assets/`, and update the in-app title-bar brand emblem in
   `apps/desktop/src/App.vue`.

## Non-goals

1. Changing the product name (`Karen`) or departing from the approved luxury
   steampunk visual direction.
2. Modifying CLI telemetry, disk scanning or removal logic.
3. Introducing external web fonts, raster `<image>` embeds or runtime script
   dependencies inside SVG icon assets.

## Design

### Logo Designer skill pipeline (`logos/`)

Follow `/Users/albert.li/.agents/skills/logo-designer/SKILL.md`:

1. **Explore (`logos/concepts/`)**: Generate 3–5 distinct 512×512 SVG concepts
   anchored in Karen's luxury steampunk instrument identity (for example:
   sector-window brass panel meter, flyball steam governor, precision pressure
   chronometer dial, and engraved brass astrolabe/treemap gauge), incorporating
   findings from `icon-design/brief.md`. Build `logos/preview.html` with
   light/dark background toggle.
2. **Refine (`logos/iterations/`)**: Iterate on the chosen direction in
   `logos/iterations/iteration-N.svg` with the 64px / 32px / 16px Favicon Size
   Check strip in `logos/preview.html` so thin ticks or low-contrast details are
   simplified or thickened before export.
3. **Export (`logos/export/`)**: Copy the approved SVG to
   `logos/export/logo.svg` (and `logos/export/icon.svg` if applicable) and run
   `bash /Users/albert.li/.agents/skills/logo-designer/scripts/export.sh logos/export/logo.svg logos/export/`
   to produce `logo-16.png`, `logo-32.png`, `logo-48.png`, `logo-192.png`,
   `logo-512.png`, `logo-1024.png` and `logo-2048.png`.
4. **Repo integration (`apps/desktop/src-tauri/icons/`, `assets/`, `apps/desktop/src/App.vue`)**:
   - Update `apps/desktop/src-tauri/icons/icon.png` (and platform bundle icons
     referenced by `apps/desktop/src-tauri/tauri.conf.json`, plus the tray icon
     used by RFC 0012).
   - Place the discoverable repository icon asset in `assets/icon.svg` (and/or
     `assets/icon.png`) and reference it in `README.md`.
   - Replace the generic header emblem in `apps/desktop/src/App.vue` with the
     redesigned inline SVG mark so the window chrome, app icon and top-bar tray
     icon share one cohesive identity.

## Acceptance

1. `logos/concepts/`, `logos/iterations/`, `logos/preview.html` and
   `logos/export/` contain valid, self-contained SVG and exported PNGs (`16`
   through `2048` px) produced by the `logo-designer` skill workflow.
2. The icon silhouette and primary needle/governor mass remain clearly legible
   at `64px`, `32px` and `16px`.
3. `apps/desktop/src-tauri/icons/icon.png` (and configured bundle/tray icons),
   `assets/icon.svg`, `README.md` and the `App.vue` title-bar emblem are updated
   and verified with `pnpm build` and `pnpm tauri`.
