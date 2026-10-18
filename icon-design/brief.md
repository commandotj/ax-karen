# Icon brief: Karen

Product: cross-platform system monitor and disk-space explorer (Rust CLI + Tauri/Vue desktop) · Audience: desktop power users who watch machine resources · Category neighbors: Activity-Monitor bar charts, speedometer gauges, CPU-chip squares, terminal `>_` prompts, pie/treemap mosaics, settings gears.

Palette note: the product brief asks for "brass on dark enamel". House palette
accommodates this without override — the dark grey background doubles as the
enamel plate, glyph engraving stays off-white, and brass is the single allowed
accent (bezel + needle). Flat tones, no gradients.

## Concepts

### 1. Panel meter  (axis: vertical)
Depicts: an analog panel meter
Metaphor: the machine's health read through a brass instrument — the same
instrument family as Karen's in-app Runtime gauges.
Gestalt device: figure-ground — a fan-shaped (sector) window cut out of a solid
brass face shows the dark enamel through the opening.
Mass: solid rounded-square bezel with the sector window knocked out of the
mass; needle and hub sit inside the window. One closed contour, detail cut
from the mass.
16px risk: engraved tick marks die first; the sector silhouette, needle and
bezel corners survive.
Distinct from: speedometer (sector window ≈100°, not a full circle); activity
monitor (no bars); CPU chip (no pins/square die).
Not a UI glyph: no system control uses a sector-dial silhouette.

### 2. Steam governor  (axis: vertical)
Depicts: a flyball governor
Metaphor: the regulator that keeps a steam machine within safe limits — the
literal job of a system monitor. Also replaces the gear emblem currently in
the app header (a settings-gear silhouette is a UI button, not a product
mark).
Gestalt device: proximity — the two flyballs frame the spindle and read as one
instrument.
Mass: solid spindle, two solid balls, stepped base plate; negative space cut
between the arms.
16px risk: the thin spindle dies; balls + base survive if drawn chunky.
Distinct from: settings gear (no teeth; masses hang outside a ring rather than
forming one); no monitoring product uses a governor.
Not a UI glyph: checked against settings gear and refresh — silhouette shares
nothing with either.

### 3. Pressure gauge  (axis: vertical)
Depicts: a pressure gauge
Metaphor: live system pressure at a glance.
Gestalt device: closure — a ring of short engraved ticks implies the full dial
circle.
Mass: solid brass bezel ring, filled dial face, needle and hub.
16px risk: closure gaps must stay tiny to complete at favicon size; ticks die,
ring + needle survive.
Distinct from: speedometer cliché only by the thick bezel-ring silhouette and
the needle parked at 12 o'clock — weakest distinctiveness of the three.
Not a UI glyph: no system control is a round gauge.

## Palette

Background `#2A2A2E` (dark enamel) · Glyph `#E8E8EA` (engraved scales, dial
markings) · Accent brass `#C08A3E` (bezel, needle, hub)

## Recommendation

**Panel meter.** It is the silhouette of the instruments Karen already renders
inside the app, survives 16px with the most body (heaviest ink, sector window
stays legible), and no competitor in the monitoring category wears a
sector-window mark. Steam governor is the boldest alternative if a more
literal steampunk reference is wanted.
