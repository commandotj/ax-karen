<script setup lang="ts">
import { computed } from "vue";
import { chartPath, formatBytes, formatRate, meterStyle, volumePercent } from "../format";
import { useI18n } from "../i18n";
import type { BatteryInfo, CpuProcess, MemoryProcess, NetworkInfo, Volume } from "../types";

const { t } = useI18n();

const props = defineProps<{
  cpu: number | null;
  cpuSamples: number[];
  cpuProcesses: CpuProcess[];
  cpuProcessesAvailable: boolean;
  memoryUsed: number | null;
  memoryTotal: number | null;
  memoryProcesses: MemoryProcess[];
  memoryProcessesAvailable: boolean;
  battery: BatteryInfo;
  network: NetworkInfo;
  volumes: Volume[];
}>();

defineEmits<{
  navigate: [tab: "memory" | "battery" | "network"];
  selectVolume: [path: string];
}>();

const usedMemoryPercent = computed(() =>
  props.memoryTotal ? Math.min(100, ((props.memoryUsed ?? 0) / props.memoryTotal) * 100) : 0,
);
const visibleCpuProcesses = computed(() => props.cpuProcesses.slice(0, 8));
</script>

<template>
  <section class="view metric-view runtime-view">
    <div class="view-heading"><h1>{{ t("nav.runtime") }}</h1></div>
    <div class="metric-console runtime-console">
      <div class="metric-main">
        <span class="metric-caption">{{ t("runtime.cpuLoad") }}</span>
        <strong class="metric-value">{{ cpu ?? "—" }}<small>%</small></strong>
        <div class="metric-ruler"><i :style="{ width: `${cpu ?? 0}%` }"></i></div>
        <div class="metric-state">
          <span>LOAD CONDITION</span>
          <b>{{ cpu === null ? "INITIALIZING" : cpu > 85 ? "HIGH LOAD" : cpu > 50 ? "ACTIVE" : "NOMINAL" }}</b>
        </div>
      </div>
      <div class="chart-console">
        <div class="chart-heading">
          <span>PROCESSOR LOAD</span><span>3 SECOND SAMPLE <i>/</i> 4.5 MIN WINDOW</span>
        </div>
        <svg class="history-chart" viewBox="0 0 1000 100" preserveAspectRatio="none" role="img" aria-label="Recent processor load history">
          <path class="chart-grid" d="M0 1H1000 M0 50H1000 M0 99H1000"/>
          <path v-if="cpuSamples.length > 1" class="chart-area" :d="`${chartPath(cpuSamples, 100)} L1000,100 L0,100 Z`"/>
          <path class="chart-line cpu-line" :d="chartPath(cpuSamples, 100)"/>
        </svg>
        <div class="chart-axis"><span>−4.5 MIN</span><span>−2 MIN</span><span>NOW</span></div>
        <div class="process-ledger">
          <div class="process-ledger-heading"><span>CPU BY PROCESS</span><span>100% = ONE CORE</span></div>
          <p v-if="!cpuProcessesAvailable" class="process-empty">SAMPLING BASELINE — PROCESS LOAD WARMING UP</p>
          <p v-else-if="!visibleCpuProcesses.length" class="process-empty">NO PROCESS CPU READINGS AVAILABLE</p>
          <div v-else class="process-rows">
            <div v-for="process in visibleCpuProcesses" :key="process.pid" class="process-row">
              <span class="process-name" :title="process.name">{{ process.name }}</span>
              <span class="process-pid">PID {{ process.pid }}</span>
              <b>{{ process.cpu_percent.toFixed(1) }}%</b>
            </div>
          </div>
        </div>
      </div>
      <div class="runtime-memory">
        <span class="metric-caption">{{ t("runtime.memoryLoad") }}</span>
        <strong>{{ memoryUsed === null ? "—" : formatBytes(memoryUsed) }}</strong>
        <div class="metric-ruler memory-ruler"><i :style="{ width: `${usedMemoryPercent}%` }"></i></div>
        <div class="runtime-memory-stats">
          <span>{{ t("memory.used") }} <b>{{ usedMemoryPercent.toFixed(0) }}%</b></span>
          <span>{{ t("memory.total") }} <b>{{ memoryTotal === null ? "—" : formatBytes(memoryTotal) }}</b></span>
          <span>{{ t("memory.free") }} <b>{{ memoryTotal === null ? "—" : formatBytes(memoryTotal - (memoryUsed ?? 0)) }}</b></span>
        </div>
        <div class="runtime-process-mini">
          <span class="metric-caption">RESIDENT MEMORY LEADERS</span>
          <div v-for="process in memoryProcesses.slice(0, 3)" :key="process.pid">
            <span>{{ process.name }}</span>
            <b>{{ formatBytes(process.memory_bytes) }}</b>
          </div>
          <p v-if="!memoryProcessesAvailable">PROCESS SAMPLE UNAVAILABLE</p>
        </div>
        <button class="runtime-memory-link" @click="$emit('navigate', 'memory')">{{ t("runtime.memoryDetail") }} <span>↗</span></button>
      </div>
      <div class="runtime-aux">
        <button class="runtime-mini-instrument runtime-battery-card" @click="$emit('navigate', 'battery')">
          <span class="metric-caption">{{ t("nav.battery") }}</span>
          <strong>{{ battery.present ? `${battery.percent ?? "—"}%` : t("battery.acPower") }}</strong>
          <small>{{ battery.present ? `${(battery.state ?? "unknown").toUpperCase()} · ${battery.power_watts != null ? `${battery.power_watts.toFixed(1)} W` : "— W"}` : t("battery.acPower") }}</small>
          <span class="runtime-mini-open">{{ t("runtime.batteryDetail") }} ↗</span>
        </button>
        <button class="runtime-mini-instrument runtime-network-card" @click="$emit('navigate', 'network')">
          <span class="metric-caption">{{ t("runtime.netThroughput") }}</span>
          <strong>RX {{ formatRate(network.rx_bytes_per_sec) }}</strong>
          <small>TX {{ formatRate(network.tx_bytes_per_sec) }} · {{ network.interfaces.length }} IFACES</small>
          <span class="runtime-mini-open">{{ t("runtime.networkDetail") }} ↗</span>
        </button>
      </div>
    </div>
    <section class="runtime-volumes">
      <div class="instrument-section-heading">
        <span>VOLUME CAPACITY INSTRUMENTS</span>
        <span>{{ volumes.length.toString().padStart(2, "0") }} MOUNTED</span>
      </div>
      <div class="runtime-volume-grid">
        <button v-for="(volume, index) in volumes" :key="volume.mount_point" class="runtime-volume" @click="$emit('selectVolume', volume.mount_point)">
          <span class="runtime-volume-index">{{ String(index + 1).padStart(2, "0") }}</span>
          <span class="runtime-volume-name">{{ volume.mount_point }}</span>
          <span class="runtime-volume-device">{{ volume.device || "LOCAL STORAGE" }}</span>
          <strong>{{ volumePercent(volume).toFixed(0) }}<small>% USED</small></strong>
          <span class="runtime-volume-track" :style="meterStyle(volumePercent(volume))"><i></i></span>
          <span class="runtime-volume-capacity">{{ formatBytes((volume.total_bytes ?? 0) - (volume.free_bytes ?? 0)) }} USED <b>{{ volume.available_bytes == null ? "—" : formatBytes(volume.available_bytes) }} FREE</b></span>
          <span class="runtime-volume-open">OPEN SPACE ↗</span>
        </button>
        <p v-if="!volumes.length" class="process-empty">WAITING FOR MOUNTED VOLUME TELEMETRY</p>
      </div>
    </section>
    <div class="panel-status">
      <span class="calibration">✥</span> LIVE SYSTEM TELEMETRY <i>·</i> CPU · MEMORY · BATTERY · NETWORK · {{ volumes.length }} VOLUMES <i>·</i> UPDATED EVERY 3 SECONDS
    </div>
  </section>
</template>
