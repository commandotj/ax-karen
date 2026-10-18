<script setup lang="ts">
import { computed } from "vue";
import DataTable from "../components/DataTable.vue";
import { chartPath, formatBytes, formatRate } from "../format";
import type { NetworkInfo } from "../types";

const props = defineProps<{
  network: NetworkInfo;
  networkRxSamples: number[];
  networkTxSamples: number[];
}>();

const networkPeakRate = computed(() =>
  Math.max(1024, ...props.networkRxSamples, ...props.networkTxSamples),
);

const columns = ["RANK / INTERFACE", "RX RATE", "TX RATE", "TOTAL RX / TX", "PACKETS / ERR"] as const;
</script>

<template>
  <section class="view metric-view network-view">
    <div class="view-heading">
      <h1>Network Throughput</h1>
      <span class="volume-count">RX {{ formatRate(network.rx_bytes_per_sec) }} · TX {{ formatRate(network.tx_bytes_per_sec) }}</span>
    </div>
    <div class="metric-console memory-console network-console">
      <div class="metric-main">
        <span class="metric-caption">EXTERNAL THROUGHPUT</span>
        <strong class="metric-value memory-value">{{ formatRate(network.rx_bytes_per_sec) }}</strong>
        <div class="metric-state"><span>TRANSMIT RATE (TX)</span><b>{{ formatRate(network.tx_bytes_per_sec) }}</b></div>
        <div class="metric-state"><span>CUMULATIVE RECEIVED</span><b>{{ formatBytes(network.total_rx_bytes) }}</b></div>
        <div class="metric-state"><span>CUMULATIVE TRANSMITTED</span><b>{{ formatBytes(network.total_tx_bytes) }}</b></div>
      </div>
      <div class="chart-console">
        <div class="chart-heading">
          <span>DUAL-CHANNEL RX / TX TRACE</span><span>PEAK SCALE {{ formatRate(networkPeakRate) }}</span>
        </div>
        <svg class="history-chart" viewBox="0 0 1000 100" preserveAspectRatio="none" role="img" aria-label="Recent network receive and transmit history">
          <path class="chart-grid" d="M0 1H1000 M0 50H1000 M0 99H1000"/>
          <path v-if="networkRxSamples.length > 1" class="chart-area memory-area" :d="`${chartPath(networkRxSamples, networkPeakRate)} L1000,100 L0,100 Z`"/>
          <path class="chart-line memory-line" :d="chartPath(networkRxSamples, networkPeakRate)"/>
          <path class="chart-line cpu-line" :d="chartPath(networkTxSamples, networkPeakRate)"/>
        </svg>
        <div class="chart-axis"><span>−4.5 MIN</span><span>−2 MIN</span><span>NOW</span></div>
      </div>
    </div>
    <DataTable
      class="memory-process-console network-interface-console"
      :columns="columns"
      :items="network.interfaces"
      heading-class="network-table-heading"
      row-class="network-interface-row"
      :empty="!network.interfaces.length"
      empty-message="NO NETWORK INTERFACES DETECTED"
      section-title="NETWORK INTERFACES"
      section-subtitle="SORTED BY LIVE THROUGHPUT · TOP 20"
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
        <span class="process-pid">{{ iface.packets_rx + iface.packets_tx }} pkts · {{ iface.errors_rx + iface.errors_tx }} err</span>
      </template>
    </DataTable>
    <div class="panel-status">
      <span class="calibration">✥</span> NETWORK TELEMETRY <i>·</i> {{ network.interfaces.length }} INTERFACES <i>·</i> 3 SECOND SAMPLE
    </div>
  </section>
</template>
