# RFC 0010: Add battery telemetry and Battery tab (child of 0001)

**Status:** Draft

**Parent:** [0001](completed/0001-karen-system-resource-app.md)

## Summary

Add read-only battery and power-supply telemetry to Karen's CLI monitor stream
and present it through a dedicated **Battery** tab and a **Runtime** console
instrument. Keep hardware sampling in a new `crates/monitor-battery/` crate
orchestrated by `crates/monitor-core/` and streamed via `karen --stream-json`,
while the Vue 3 desktop shell renders charge gauges, wattage traces, capacity
health and explicit desktop/no-battery states in Karen's luxury steampunk
visual language.

## Problem

Karen currently monitors processor load, physical memory and mounted disk space,
but provides no visibility into battery charge, power draw, estimated runtime
or cell health on portable machines. Users who rely on Karen's Power Efficiency
scan presets (`Miser`, `Balanced`, `Aggressive`, `Drain My Battery`) cannot
observe actual battery state, discharge wattage or remaining capacity inside
the monitor console.

## Goals

1. Sample read-only battery telemetry across macOS, Linux and Windows in a
   dedicated `crates/monitor-battery/` workspace crate without elevated
   privileges.
2. Expose normalized battery readings through `monitor-core::ResourceSnapshot`
   and the CLI `resources` stream event on the standard resource sampling
   cadence.
3. Add a **Battery** tab to the desktop navigation bar and a clickable Battery
   instrument on the **Runtime** overview console that navigates to the
   **Battery** tab.
4. Visualize charge percentage, charging/discharging/full state, instantaneous
   power rate (watts), estimated time to empty or full, current vs design
   capacity (`Wh`), battery health percentage, voltage, temperature (when
   available) and cycle count using engraved brass gauges, live traces and
   exact numeric readouts.
5. Render an explicit, honest `No battery detected (AC power)` state on desktop
   systems or virtual machines without fabricating readings.

## Non-goals

1. Modifying operating-system power profiles, sleep timers or charging limits.
2. Per-process energy impact estimation or privileged power- rail counters.
3. Historical database persistence, background alerts or remote telemetry.
4. Network monitoring (RFC 0011), top-bar tray integration (RFC 0012) or icon
   redesign (RFC 0013).

## Design

### CLI and core telemetry (`crates/monitor-battery/`, `crates/monitor-core/`, `apps/cli/`)

Create `crates/monitor-battery/` in the Cargo workspace to query platform
battery state (via `starship-battery` or platform power APIs: IOKit `IOPowerSources`
on macOS, `/sys/class/power_supply` on Linux, and Windows battery IOCTLs/WMI).
Normalize readings into a serializable `Snapshot`:

- `present: bool` — whether at least one system battery is detected.
- `state: Option<BatteryState>` — `Charging`, `Discharging`, `Full`, `Empty` or
  `Unknown`.
- `percent: Option<u32>` — state of charge clamped to `0..=100`.
- `power_watts: Option<f32>` — non-negative charge or discharge rate in watts.
- `energy_wh: Option<f32>`, `energy_full_wh: Option<f32>`,
  `energy_design_wh: Option<f32>` — current, full-charge and factory-design
  energy capacities in watt-hours.
- `health_percent: Option<u32>` — derived from `energy_full_wh / energy_design_wh`
  clamped to finite percentage values.
- `voltage_volts: Option<f32>`, `temperature_celsius: Option<f32>`,
  `cycle_count: Option<u32>`.
- `time_to_empty_secs: Option<u64>`, `time_to_full_secs: Option<u64>`.

Sanitize any non-finite, negative or unreadable platform fields to `None`.
Extend `monitor_core::ResourceSnapshot` with `battery: monitor_battery::Snapshot`
and serialize a `battery` object inside the `resources` event in
`apps/cli/src/stream_json.rs`. Preserve Tauri as a pure pass-through shell with
no monitor crate dependencies.

### Desktop presentation (`apps/desktop/src/App.vue`, `apps/desktop/src/style.css`)

- **Runtime console**: Add a compact Battery instrument panel alongside CPU and
  Memory showing charge percentage, charging/discharging state lamp, live power
  draw (`W`) and remaining time or AC status. Clicking the instrument switches
  to the **Battery** tab.
- **Battery tab**: Render a dedicated luxury steampunk instrument console
  containing:
  1. Primary analog charge dial and segmented enamel bar with state indicator
     (`CHARGING`, `DISCHARGING`, `FULL`, `NO BATTERY`).
  2. Live charge percentage and wattage history traces across recent samples.
  3. Capacity & Health instrument comparing current full capacity (`Wh`) against
     factory design capacity (`Wh`), health percentage and cycle count.
  4. Electrical & runtime readout ledger showing voltage (`V`), power rate
     (`W`), temperature (`°C` when exposed) and estimated time remaining.

## Acceptance

1. `cargo xtask lint` and `cargo xtask test` pass with unit tests in
   `crates/monitor-battery/` covering normal readings, non-finite sanitization,
   derived health percentage and no-battery fallback.
2. `karen --stream-json` includes the `battery` payload in `resources` events
   without breaking existing CPU, memory or disk fields.
3. On systems with a battery, both **Runtime** and the **Battery** tab display
   live charge percentage, state, power draw, capacity health and history
   traces; on desktop systems without a battery, both views clearly report that
   no battery is present.
4. Clicking the Runtime battery instrument opens the **Battery** tab; keyboard
   navigation, accessible labels, custom window chrome and luxury steampunk
   styling remain intact at narrow and wide viewport sizes.
