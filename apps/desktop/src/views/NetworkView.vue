<script setup lang="ts">
import { computed, ref } from "vue";
import DataTable from "../components/DataTable.vue";
import { chartPath, formatBytes, formatRate } from "../format";
import { useI18n } from "../i18n";
import type { NetworkConnection, NetworkInfo, NetworkProcess } from "../types";

const { t, text } = useI18n();

const props = defineProps<{
  network: NetworkInfo;
  networkRxSamples: number[];
  networkTxSamples: number[];
}>();

const networkPeakRate = computed(() =>
  Math.max(1024, ...props.networkRxSamples, ...props.networkTxSamples),
);

type SubTab = "apps" | "interfaces" | "astrolabe";
const userSelectedTab = ref<SubTab | null>(null);

const activeSubTab = computed<SubTab>({
  get: () => {
    if (userSelectedTab.value) return userSelectedTab.value;
    if (props.network.processes && props.network.processes.length > 0) return "apps";
    return "interfaces";
  },
  set: (val: SubTab) => {
    userSelectedTab.value = val;
  },
});

const processes = computed<NetworkProcess[]>(
  () => props.network.processes ?? [],
);
const connections = computed<NetworkConnection[]>(
  () => props.network.connections ?? [],
);

const interfaceColumns = computed(() => [
  text("RANK / INTERFACE"), text("RX RATE"), text("TX RATE"),
  text("TOTAL RX / TX"), text("PACKETS / ERR"),
]);

const processColumns = computed(() => [
  text("RANK / PROCESS"), "PID", text("SOCKETS"), text("REMOTE HOSTS"), text("PROTOCOLS"),
]);

const connectionColumns = computed(() => [
  text("SERVICE / PROTO"), text("PROCESS / PID"), text("LOCAL ENDPOINT"),
  text("REMOTE ENDPOINT"), text("STATE / NET"),
]);

const lanConnections = computed(() =>
  connections.value.filter((c) => c.is_lan),
);
const wanConnections = computed(() =>
  connections.value.filter((c) => !c.is_lan),
);
const establishedCount = computed(
  () => connections.value.filter((c) => c.state === "ESTABLISHED").length,
);

const ASTROLABE_CX = 400;
const ASTROLABE_CY = 200;
const ASTROLABE_R_LAN = 85;
const ASTROLABE_R_WAN = 150;

type AstrolabeNode = {
  conn: NetworkConnection;
  x: number;
  y: number;
  isLan: boolean;
};

const astrolabeNodes = computed<AstrolabeNode[]>(() => {
  const nodes: AstrolabeNode[] = [];
  const lans = lanConnections.value;
  const wans = wanConnections.value;

  lans.forEach((conn, idx) => {
    const angle = (idx * 2 * Math.PI) / Math.max(1, lans.length) - Math.PI / 2;
    nodes.push({
      conn,
      x: ASTROLABE_CX + ASTROLABE_R_LAN * Math.cos(angle),
      y: ASTROLABE_CY + ASTROLABE_R_LAN * Math.sin(angle),
      isLan: true,
    });
  });

  wans.forEach((conn, idx) => {
    const angle = (idx * 2 * Math.PI) / Math.max(1, wans.length) - Math.PI / 2;
    nodes.push({
      conn,
      x: ASTROLABE_CX + ASTROLABE_R_WAN * Math.cos(angle),
      y: ASTROLABE_CY + ASTROLABE_R_WAN * Math.sin(angle),
      isLan: false,
    });
  });

  return nodes;
});
</script>

<template>
  <section class="view metric-view network-view">
    <div class="view-heading">
      <h1>{{ t("network.title") }}</h1>
      <span class="volume-count">RX {{ formatRate(network.rx_bytes_per_sec) }} · TX {{ formatRate(network.tx_bytes_per_sec) }}</span>
    </div>

    <div class="metric-console memory-console network-console">
      <div class="metric-main">
        <span class="metric-caption">{{ t("runtime.netThroughput") }}</span>
        <strong class="metric-value memory-value">{{ formatRate(network.rx_bytes_per_sec) }}</strong>
        <div class="metric-state"><span>{{ text("TRANSMIT RATE (TX)") }}</span><b>{{ formatRate(network.tx_bytes_per_sec) }}</b></div>
        <div class="metric-state"><span>{{ t("network.totalRx") }}</span><b>{{ formatBytes(network.total_rx_bytes) }}</b></div>
        <div class="metric-state"><span>{{ t("network.totalTx") }}</span><b>{{ formatBytes(network.total_tx_bytes) }}</b></div>
      </div>
      <div class="chart-console">
        <div class="chart-heading">
          <span>{{ text("DUAL-CHANNEL RX / TX TRACE") }}</span><span>{{ text("PEAK SCALE") }} {{ formatRate(networkPeakRate) }}</span>
        </div>
        <svg class="history-chart" viewBox="0 0 1000 100" preserveAspectRatio="none" role="img" :aria-label="text('Recent network receive and transmit history')">
          <path class="chart-grid" d="M0 1H1000 M0 50H1000 M0 99H1000"/>
          <path v-if="networkRxSamples.length > 1" class="chart-area memory-area" :d="`${chartPath(networkRxSamples, networkPeakRate)} L1000,100 L0,100 Z`"/>
          <path class="chart-line memory-line" :d="chartPath(networkRxSamples, networkPeakRate)"/>
          <path class="chart-line cpu-line" :d="chartPath(networkTxSamples, networkPeakRate)"/>
        </svg>
        <div class="chart-axis"><span>{{ text("−4.5 MIN") }}</span><span>{{ text("−2 MIN") }}</span><span>{{ text("NOW") }}</span></div>
      </div>
    </div>

    <div class="network-subnav">
      <button
        type="button"
        class="network-nav-btn"
        :class="{ 'is-active': activeSubTab === 'apps' }"
        data-testid="subtab-apps"
        @click="activeSubTab = 'apps'"
      >
        <span class="nav-brass-dot">◆</span>
        {{ t("network.processLedger") }} ({{ processes.length }})
      </button>
      <button
        type="button"
        class="network-nav-btn"
        :class="{ 'is-active': activeSubTab === 'interfaces' }"
        data-testid="subtab-interfaces"
        @click="activeSubTab = 'interfaces'"
      >
        <span class="nav-brass-dot">◈</span>
        {{ t("network.interfaces") }} ({{ network.interfaces.length }})
      </button>
      <button
        type="button"
        class="network-nav-btn"
        :class="{ 'is-active': activeSubTab === 'astrolabe' }"
        data-testid="subtab-astrolabe"
        @click="activeSubTab = 'astrolabe'"
      >
        <span class="nav-brass-dot">❖</span>
        {{ t("network.astrolabeTitle") }} ({{ connections.length }})
      </button>
    </div>

    <div v-if="activeSubTab === 'apps'" class="network-tab-pane network-apps-pane">
      <DataTable
        class="memory-process-console network-process-table"
        :columns="processColumns"
        :items="processes"
        heading-class="network-process-heading"
        row-class="network-process-row"
        :empty="!processes.length"
        :empty-message="text('NO ACTIVE NETWORK PROCESSES DETECTED')"
        :section-title="text('PROCESS NETWORK LEDGER')"
        :section-subtitle="text('UNPRIVILEGED SOCKET ATTRIBUTION · TOP 20')"
        :key-prop="(proc) => proc.pid"
      >
        <template #row="{ item: proc, index }">
          <span class="detail-process-identity">
            <i>{{ String(index + 1).padStart(2, "0") }}</i>
            <b :title="proc.name">{{ proc.name }}</b>
          </span>
          <span class="process-pid">{{ proc.pid }}</span>
          <span class="process-pid">{{ proc.connection_count }} {{ text("sockets") }}</span>
          <span class="process-pid" :title="proc.remote_hosts.join(', ')">
            {{ proc.remote_hosts.length ? `${proc.remote_hosts.length} ${text("remotes")}` : text("local only") }}
          </span>
          <span class="process-pid">
            <span v-for="proto in proc.protocols" :key="proto" class="proto-badge">{{ proto }}</span>
          </span>
        </template>
      </DataTable>

      <DataTable
        v-if="connections.length"
        class="memory-process-console network-connection-table"
        :columns="connectionColumns"
        :items="connections"
        heading-class="network-connection-heading"
        row-class="network-connection-row"
        :empty="!connections.length"
        :empty-message="text('NO ACTIVE CONNECTIONS')"
        :section-title="text('ACTIVE SOCKET CONNECTIONS')"
        :section-subtitle="text('BOUNDED ENDPOINT ROSTER · TOP 50')"
        :key-prop="(conn, i) => `${conn.local_port}-${conn.remote_port ?? 0}-${i}`"
      >
        <template #row="{ item: conn }">
          <span class="detail-process-identity">
            <span class="service-badge" :class="{ 'is-known': Boolean(conn.service_label) }">
              {{ conn.service_label || conn.protocol }}
            </span>
          </span>
          <span class="process-pid" :title="conn.process_name || text('untracked')">
            {{ conn.process_name || (conn.pid != null ? `pid-${conn.pid}` : 'system') }}
          </span>
          <span class="process-pid">{{ conn.local_addr }}:{{ conn.local_port }}</span>
          <span class="process-pid">
            {{ conn.remote_addr ? `${conn.remote_addr}:${conn.remote_port}` : '—' }}
          </span>
          <span class="process-pid">
            <span class="state-badge">{{ text(conn.state) }}</span>
            <span class="net-scope-badge" :class="conn.is_lan ? 'scope-lan' : 'scope-wan'">
              {{ conn.is_lan ? 'LAN' : 'WAN' }}
            </span>
          </span>
        </template>
      </DataTable>
    </div>

    <div v-else-if="activeSubTab === 'interfaces'" class="network-tab-pane">
      <DataTable
        class="memory-process-console network-interface-console"
        :columns="interfaceColumns"
        :items="network.interfaces"
        heading-class="network-table-heading"
        row-class="network-interface-row"
        :empty="!network.interfaces.length"
        :empty-message="text('NO NETWORK INTERFACES DETECTED')"
        :section-title="text('PHYSICAL & VIRTUAL INTERFACES')"
        :section-subtitle="text('SORTED BY LIVE THROUGHPUT · TOP 20')"
        :key-prop="(iface) => iface.name"
      >
        <template #row="{ item: iface, index }">
          <span class="detail-process-identity">
            <i>{{ String(index + 1).padStart(2, "0") }}</i>
            <b :title="iface.name">{{ iface.name }}</b>
          </span>
          <span class="process-pid">{{ formatRate(iface.rx_bytes_per_sec) }}</span>
          <span class="process-pid">{{ formatRate(iface.tx_bytes_per_sec) }}</span>
          <span class="process-pid">{{ formatBytes(iface.total_rx_bytes) }} / {{ formatBytes(iface.total_tx_bytes) }}</span>
          <span class="process-pid">{{ iface.packets_rx + iface.packets_tx }} {{ text("pkts") }} · {{ iface.errors_rx + iface.errors_tx }} {{ text("err") }}</span>
        </template>
      </DataTable>
    </div>

    <div v-else class="astrolabe-console">
      <div class="astrolabe-header">
        <span>{{ text("STEAMPUNK CONNECTION ASTROLABE") }}</span>
        <span>{{ text("CELESTIAL SOCKET TOPOLOGY") }}</span>
      </div>
      <div class="astrolabe-viewport">
        <svg class="astrolabe-svg" viewBox="0 0 800 400" preserveAspectRatio="xMidYMid meet" role="img" :aria-label="text('Connection Astrolabe')">
          <defs>
            <radialGradient id="hubGrad" cx="50%" cy="50%" r="50%">
              <stop offset="0%" stop-color="#ffe8a3"/>
              <stop offset="60%" stop-color="#b88b3b"/>
              <stop offset="100%" stop-color="#3d2a13"/>
            </radialGradient>
            <radialGradient id="lanGlow" cx="50%" cy="50%" r="50%">
              <stop offset="0%" stop-color="#ffd56b"/>
              <stop offset="100%" stop-color="#ffd56b00"/>
            </radialGradient>
            <radialGradient id="wanGlow" cx="50%" cy="50%" r="50%">
              <stop offset="0%" stop-color="#6be3ff"/>
              <stop offset="100%" stop-color="#6be3ff00"/>
            </radialGradient>
          </defs>

          <!-- Outer Brass Reticle & Cardinal Axes -->
          <circle :cx="ASTROLABE_CX" :cy="ASTROLABE_CY" r="170" class="astrolabe-ring-outer"/>
          <circle :cx="ASTROLABE_CX" :cy="ASTROLABE_CY" :r="ASTROLABE_R_WAN" class="astrolabe-ring-wan"/>
          <circle :cx="ASTROLABE_CX" :cy="ASTROLABE_CY" :r="ASTROLABE_R_LAN" class="astrolabe-ring-lan"/>
          <circle :cx="ASTROLABE_CX" :cy="ASTROLABE_CY" r="30" class="astrolabe-ring-core"/>

          <!-- Crosshairs -->
          <line :x1="ASTROLABE_CX - 175" :y1="ASTROLABE_CY" :x2="ASTROLABE_CX + 175" :y2="ASTROLABE_CY" class="astrolabe-axis"/>
          <line :x1="ASTROLABE_CX" :y1="ASTROLABE_CY - 175" :x2="ASTROLABE_CX" :y2="ASTROLABE_CY + 175" class="astrolabe-axis"/>

          <!-- Cardinal Markings -->
          <text :x="ASTROLABE_CX" :y="ASTROLABE_CY - 180" class="astrolabe-cardinal">N · 000°</text>
          <text :x="ASTROLABE_CX + 190" :y="ASTROLABE_CY + 4" class="astrolabe-cardinal">E · 090°</text>
          <text :x="ASTROLABE_CX" :y="ASTROLABE_CY + 190" class="astrolabe-cardinal">S · 180°</text>
          <text :x="ASTROLABE_CX - 190" :y="ASTROLABE_CY + 4" class="astrolabe-cardinal">W · 270°</text>

          <!-- Ring Labels -->
          <text :x="ASTROLABE_CX" :y="ASTROLABE_CY - ASTROLABE_R_LAN - 6" class="astrolabe-ring-label">{{ text("LAN PERIMETER") }} ({{ lanConnections.length }})</text>
          <text :x="ASTROLABE_CX" :y="ASTROLABE_CY - ASTROLABE_R_WAN - 6" class="astrolabe-ring-label">{{ text("WAN CELESTIAL ORBIT") }} ({{ wanConnections.length }})</text>

          <!-- Vectors from Center to Nodes -->
          <line
            v-for="(node, i) in astrolabeNodes"
            :key="`vec-${i}`"
            :x1="ASTROLABE_CX"
            :y1="ASTROLABE_CY"
            :x2="node.x"
            :y2="node.y"
            class="astrolabe-vector"
            :class="node.isLan ? 'vector-lan' : 'vector-wan'"
          />

          <!-- Node Dots -->
          <g v-for="(node, i) in astrolabeNodes" :key="`dot-${i}`" class="astrolabe-node">
            <circle
              :cx="node.x"
              :cy="node.y"
              r="6"
              class="node-glow"
              :fill="node.isLan ? 'url(#lanGlow)' : 'url(#wanGlow)'"
            />
            <circle
              :cx="node.x"
              :cy="node.y"
              r="3.5"
              class="node-core"
              :class="node.isLan ? 'node-lan' : 'node-wan'"
            >
              <title>{{ node.conn.process_name || 'proc' }} → {{ node.conn.remote_addr || 'local' }}:{{ node.conn.remote_port || node.conn.local_port }} ({{ node.conn.service_label || node.conn.protocol }})</title>
            </circle>
          </g>

          <!-- Center Compass Gear Hub -->
          <circle :cx="ASTROLABE_CX" :cy="ASTROLABE_CY" r="14" fill="url(#hubGrad)" stroke="#5b4420" stroke-width="1.5"/>
          <circle :cx="ASTROLABE_CX" :cy="ASTROLABE_CY" r="5" fill="#1b140b" stroke="#e5c271" stroke-width="1"/>
          <text :x="ASTROLABE_CX" :y="ASTROLABE_CY + 26" class="astrolabe-hub-label">{{ text("LOCAL HOST") }}</text>
        </svg>
      </div>
      <div class="astrolabe-metrics">
        <div class="astrolabe-metric-card">
          <span>{{ t("network.activeSockets") }}</span>
          <b>{{ connections.length }}</b>
        </div>
        <div class="astrolabe-metric-card">
          <span>{{ text("LAN NEIGHBORS") }}</span>
          <b>{{ lanConnections.length }}</b>
        </div>
        <div class="astrolabe-metric-card">
          <span>{{ text("WAN DESTINATIONS") }}</span>
          <b>{{ wanConnections.length }}</b>
        </div>
        <div class="astrolabe-metric-card">
          <span>{{ text("ESTABLISHED") }}</span>
          <b>{{ establishedCount }}</b>
        </div>
      </div>
    </div>

    <div class="panel-status">
      <span class="calibration">✥</span>
      {{ text("NETWORK TELEMETRY") }} <i>·</i>
      {{ processes.length }} {{ text("PROCESSES") }} <i>·</i>
      {{ connections.length }} {{ text("CONNECTIONS") }} <i>·</i>
      {{ network.interfaces.length }} {{ text("INTERFACES") }} <i>·</i>
      {{ text("3 SECOND SAMPLE") }}
    </div>
  </section>
</template>
