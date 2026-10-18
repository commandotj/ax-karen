# RFC 0011: Add network telemetry and Network tab (child of 0001)

**Status:** Approved

**Parent:** [0001](completed/0001-karen-system-resource-app.md)

## Summary

Add read-only network throughput and per-interface telemetry to Karen's CLI
monitor stream and expose it through a dedicated **Network** tab and a
**Runtime** console instrument. Keep network sampling and rate calculation in a
new `crates/monitor-network/` crate orchestrated by `crates/monitor-core/` and
streamed via `karen --stream-json`, while the Vue 3 desktop shell renders
receive/transmit gauges, dual-channel live traces and a bounded interface
breakdown in Karen's luxury steampunk visual language.

## Problem

Karen currently provides no visibility into network activity. Users monitoring
system load cannot tell whether bandwidth saturation or interface traffic is
accompanying CPU, memory or disk activity, and RFC 0007 previously deferred
network telemetry to a separate RFC.

## Goals

1. Sample read-only network interface counters across macOS, Linux and Windows
   in a dedicated `crates/monitor-network/` crate using unprivileged OS APIs
   (`sysinfo::Networks`).
2. Calculate aggregate and per-interface receive (`RX`) and transmit (`TX`)
   byte rates (`bytes/sec`) from elapsed sample intervals alongside cumulative
   transfer totals and packet counters.
3. Expose bounded network telemetry in `monitor-core::ResourceSnapshot` and the
   CLI `resources` stream event.
4. Add a **Network** tab to the desktop navigation bar and a clickable Network
   instrument on the **Runtime** console that opens the **Network** tab.
5. Present RX/TX throughput dials, live dual-series history traces, cumulative
   bytes transferred and a bounded per-interface table with explicit rate and
   byte units.

## Non-goals

1. Packet sniffing, deep packet inspection (`pcap`/`BPF`), or privileged
   per-process socket attribution.
2. Firewall management, interface configuration or active remote probes
   (ping/traceroute).
3. Battery monitoring (RFC 0010), top-bar tray icon (RFC 0012) or icon redesign
   (RFC 0013).

## Design

### CLI and core telemetry (`crates/monitor-network/`, `crates/monitor-core/`, `apps/cli/`)

Create `crates/monitor-network/` in the Cargo workspace wrapping
`sysinfo::Networks` (enable the `network` feature on workspace `sysinfo`).
Track sample timestamps (`Instant`) inside `monitor_network::Sampler`:

- On the first sample (warm-up) or when elapsed time is zero, report cumulative
  byte totals (`total_rx_bytes`, `total_tx_bytes`) and set rate fields
  (`rx_bytes_per_sec`, `tx_bytes_per_sec`) to `None`, matching `monitor-cpu`
  warm-up semantics.
- On subsequent samples, compute `rx_bytes_per_sec` and `tx_bytes_per_sec` from
  the byte delta divided by elapsed seconds since the previous refresh.
- Exclude loopback interfaces (`lo`, `lo0`) from aggregate external throughput
  totals or flag them explicitly; sort interface rows descending by combined
  `rx_bytes_per_sec + tx_bytes_per_sec`, then total transferred bytes, then
  interface name for deterministic ties, bounded to at most 20 interfaces.
- Each `InterfaceUsage` entry includes `name`, `rx_bytes_per_sec`,
  `tx_bytes_per_sec`, `total_rx_bytes`, `total_tx_bytes`, `packets_rx`,
  `packets_tx`, `errors_rx` and `errors_tx`.

Extend `monitor_core::ResourceSnapshot` with `network: monitor_network::Snapshot`
and serialize `network` inside the CLI `resources` event in
`apps/cli/src/stream_json.rs`.

### Desktop presentation (`apps/desktop/src/App.vue`, `apps/desktop/src/style.css`)

- **Runtime console**: Add a Network telegraph/throughput instrument displaying
  live aggregate `RX` and `TX` rates (`B/s`, `KB/s`, `MB/s`), cumulative session
  or boot totals, active interface summary and a compact trace. Clicking the
  instrument switches to the **Network** tab.
- **Network tab**: Render a dedicated steampunk instrument panel with:
  1. Dual brass throughput gauges for Receive (`RX`) and Transmit (`TX`) rates
     plus peak rate readouts.
  2. Live dual-trace chart plotting recent `RX` and `TX` throughput history.
  3. Bounded per-interface instrument table listing interface name, live `RX/s`,
     live `TX/s`, total `RX` bytes, total `TX` bytes and packet/error counts,
     with explicit warm-up and unavailable states.

## Acceptance

1. `pnpm lint` (`node scripts/cargo-check.mjs lint`) and `pnpm test` (`node scripts/cargo-check.mjs test`) pass with unit tests in
   `crates/monitor-network/` covering warm-up `None` rates, rate calculation
   over elapsed intervals, deterministic sorting and the 20-interface bound.
2. `karen --stream-json` emits `network` data in `resources` events on macOS,
   Linux and Windows.
3. **Runtime** displays the Network throughput instrument and navigates to the
   **Network** tab on click; the **Network** tab renders live RX/TX gauges,
   traces and the bounded per-interface breakdown with explicit units.
4. Launching or switching to the **Network** tab never triggers a disk scan and
   preserves Karen's custom chrome, keyboard accessibility and steampunk design
   constraints.
