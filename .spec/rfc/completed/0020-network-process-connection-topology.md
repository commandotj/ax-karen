# RFC 0020: Network process attribution and connection topology

**Status:** Implemented

## Summary

Integrate Snitch-inspired network visibility into Karen's Network tab. Expand `monitor-network` and `monitor-core` to collect active socket connections and attribute remote endpoints to local processes using unprivileged OS connection tables. Elevate `NetworkView.vue` with a dual-mode presentation: an active process-level network ledger (using `DataTable.vue`) and a steampunk Connection Astrolabe / Radar visualization showing local host, LAN perimeter, and external endpoints. Maintain Karen's architectural invariants: zero root/sudo requirement by default, CLI-owned state and sampling, unprivileged execution, pure presentation in Vue, and 100% test coverage.

## Problem

Karen's current Network tab only reports raw physical and virtual network interfaces (`en0`, `lo0`, `utun3`) with aggregate byte and packet counters. While the CPU tab provides top per-process CPU usage and the Memory tab provides top per-process resident memory, the Network tab provides **zero visibility into which processes or applications are communicating over the network**. Users cannot tell which application is consuming bandwidth, what remote hosts their computer is talking to, or what ports are open.

Snitch (`github.com/aixisstudio/Snitch`) demonstrates the value of:
1. **Process attribution**: knowing exactly which process/PID owns each active socket and destination.
2. **Endpoint visibility**: distinguishing local loopback, LAN perimeter (gateway and local devices), and remote WAN hosts.
3. **Traffic classification**: categorizing ports and services (HTTP/S, DNS, SSH, Database, CDN, Trackers).
4. **Topology visualization**: representing network connections as an intuitive relational graph rather than disjointed raw byte counts.

However, Snitch relies on a heavy Python backend (`ctypes` -> `libpcap`, FastAPI, SQLite) and requires root/sudo privileges for raw packet sniffing. Karen must achieve comparable, practical network legibility natively in Rust without requiring root/sudo privileges, without heavy runtime dependencies, and aligned with Karen's luxury steampunk design.

## Goals

1. **Unprivileged Process Network Attribution**:
   - In `monitor-network` and `monitor-core`, enumerate active TCP and UDP sockets from the operating system without requiring root or administrative privileges (using platform socket tables / `netstat2`).
   - Correlate local listening and established ports with local PIDs and process names (using existing `sysinfo` process cache).
   - Group connections by process: report top processes by active socket count and remote endpoint diversity.
2. **Remote Endpoint & Connection Legibility**:
   - Extract remote IP, remote port, protocol (TCP/UDP), and connection state (`ESTABLISHED`, `LISTEN`, `TIME_WAIT`, `SYN_SENT`).
   - Categorize destinations into `lan` (private RFC 1918 / link-local) and `wan` (public internet), identifying the local default gateway.
   - Map well-known destination ports to service labels (HTTPS, HTTP, SSH, DNS, NTP, Postgres, Redis, etc.).
3. **Steampunk Network Presentation in `NetworkView.vue`**:
   - Provide a view mode switcher between **"PROCESS LEDGER"** (which apps are talking) and **"PHYSICAL INTERFACES"** (raw hardware counters).
   - In Process Ledger mode, reuse [`DataTable.vue`](../../apps/desktop/src/components/DataTable.vue) to display:
     - Rank index, process name, PID, active socket count, remote destination count, primary protocols, and connection states.
     - Collapsible or inspectable detail showing the exact remote endpoints for the selected process.
   - Add a Steampunk **Connection Astrolabe** visual instrument:
     - Center: Local Machine (ornate Karen brass compass).
     - Inner Perimeter: LAN / Gateway perimeter ring.
     - Outer Astrolabe Rings: Active remote destinations, color-coded by service category (brass/amber for web, cyan for infrastructure/services, copper for SSH/admin).
4. **Protocol & Invariant Adherence**:
   - Deliver data over Karen's existing `--stream-json` telemetry stream on the standard 3-second cadence.
   - Bound serialized connection lists (top 20 processes, top 50 active non-loopback connections) to prevent memory growth or stream bloat.
   - Zero root/sudo prompts: default to unprivileged OS socket table inspection.
   - Maintain 100% statement and line test coverage across all modified and new crates/modules.

## Non-goals

- Kernel-level packet sniffing requiring `sudo` or `/dev/bpf` capture.
- Active network scanning, port probing, or packet injection.
- Deep packet inspection (payload inspection, TLS decryption, or raw packet parsing).
- Firewalling, packet filtering, or connection termination (Lulu / Little Snitch blocking capabilities).
- Third-party cloud analytics or external online GeoIP lookups (all resolution remains local or cached).

## Design

### 1. Backend: Socket & Process Correlation (`monitor-network` & `monitor-core`)

Extend `crates/monitor-network`:
- Add a socket inspection engine using cross-platform native OS socket APIs (via `netstat2` or native platform bindings):
  - macOS: `libproc` / `proc_pidinfo` with `PROC_PIDLISTFDS` and `PROC_PIDFDSOCKETINFO`.
  - Linux: `/proc/net/tcp`, `/proc/net/tcp6`, `/proc/net/udp`.
  - Windows: `GetExtendedTcpTable`, `GetExtendedUdpTable`.
- Define bounded data structures:
  ```rust
  pub struct SocketConnection {
      pub pid: Option<u32>,
      pub process_name: Option<String>,
      pub protocol: Protocol, // Tcp or Udp
      pub local_addr: String,
      pub local_port: u16,
      pub remote_addr: Option<String>,
      pub remote_port: Option<u16>,
      pub state: ConnectionState, // Established, Listen, etc.
      pub service_label: Option<String>,
      pub is_lan: bool,
  }

  pub struct ProcessNetworkUsage {
      pub pid: u32,
      pub name: String,
      pub connection_count: usize,
      pub remote_hosts: Vec<String>,
      pub protocols: Vec<String>,
  }
  ```
- Correlate sockets with PIDs and process names using a shared or passed `sysinfo` process cache (matching how `monitor-cpu` and `monitor-memory` operate).
- Rank and bound:
  - Top 20 processes sorted by connection count, then unique remote hosts, then PID.
  - Top 50 active connections (prioritizing `ESTABLISHED` WAN connections over idle loopback).

Extend `crates/monitor-core`:
- Add `process_network` and `connections` fields to the `NetworkInfo` payload in the CLI `resources` stream event.

### 2. Desktop Frontend (`apps/desktop/src/views/NetworkView.vue`)

- Update `types.ts` with `ProcessNetworkUsage` and `SocketConnection` interfaces.
- In `NetworkView.vue`:
  - Preserve the top throughput meter console and dual-channel RX/TX trace chart.
  - Add a sub-navigation toggle: `[ APPS & PROCESSES (N) ]` | `[ INTERFACES (M) ]` | `[ ASTROLABE ]`.
  - In `APPS & PROCESSES` mode:
    - Render `DataTable.vue` listing processes, PIDs, active connection counts, remote destination summaries, and service tags.
    - Clicking a row highlights or expands its active remote endpoints.
  - In `ASTROLABE` mode:
    - Render a Canvas / SVG Astrolabe instrument:
      - Concentric brass astrolabe dials.
      - Core gear hub: local machine.
      - Inner ring: LAN / Gateway.
      - Outer ring: WAN endpoints distributed radially with connection vectors.
      - Tooltip inspection on hover.

## Acceptance

1. `monitor-network` exposes `sample_connections()` returning bounded, sorted process network summaries and active connections.
2. The CLI `--stream-json` `resources` event includes process network attribution and active connections.
3. The desktop Network tab displays top processes generating network activity using `DataTable.vue`.
4. Network connections are categorized by LAN vs WAN, with well-known ports labeled.
5. The interface functions entirely unprivileged (no root/sudo prompts on macOS, Linux, or Windows).
6. 100% test coverage is maintained across all Rust crates and Vue frontend modules (`pnpm lint`, `pnpm test`, `pnpm --dir apps/desktop exec vitest run --coverage`).
7. Steampunk design invariants are preserved with pixel-perfect visual hierarchy.
