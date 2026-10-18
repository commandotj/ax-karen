<script setup lang="ts">
import { computed } from "vue";
import DataTable from "../components/DataTable.vue";
import { chartPath, formatBytes, processMetricWidth } from "../format";
import { useI18n } from "../i18n";
import type { MemoryProcess } from "../types";

const { t } = useI18n();

const props = defineProps<{
  memoryUsed: number | null;
  memoryTotal: number | null;
  memorySamples: number[];
  memoryProcesses: MemoryProcess[];
  memoryProcessesAvailable: boolean;
}>();

const usedMemoryPercent = computed(() =>
  props.memoryTotal ? Math.min(100, ((props.memoryUsed ?? 0) / props.memoryTotal) * 100) : 0,
);
const largestMemoryProcess = computed(() => props.memoryProcesses[0]?.memory_bytes ?? 0);
const columns = computed(() => [t("runtime.processes"), t("network.colPid"), t("memory.used")]);
</script>

<template>
  <section class="view metric-view memory-view">
    <div class="view-heading"><h1>{{ t("nav.memory") }}</h1></div>
    <div class="metric-console memory-console">
      <div class="metric-main">
        <span class="metric-caption">{{ t("memory.systemMemory") }}</span>
        <strong class="metric-value memory-value">{{ memoryUsed === null ? "—" : formatBytes(memoryUsed) }}</strong>
        <div class="metric-ruler memory-ruler"><i :style="{ width: `${usedMemoryPercent}%` }"></i></div>
        <div class="metric-state">
          <span>{{ t("memory.total") }}</span>
          <b>{{ memoryTotal === null ? "INITIALIZING" : formatBytes(memoryTotal) }}</b>
        </div>
        <div class="metric-state">
          <span>{{ t("memory.free") }}</span>
          <b>{{ memoryTotal === null ? "—" : formatBytes(memoryTotal - (memoryUsed ?? 0)) }}</b>
        </div>
      </div>
      <div class="chart-console">
        <div class="chart-heading">
          <span>MEMORY PRESSURE</span><span>3 SECOND SAMPLE <i>/</i> 4.5 MIN WINDOW</span>
        </div>
        <svg class="history-chart" viewBox="0 0 1000 100" preserveAspectRatio="none" role="img" aria-label="Recent memory use history">
          <path class="chart-grid" d="M0 1H1000 M0 50H1000 M0 99H1000"/>
          <path v-if="memorySamples.length > 1" class="chart-area memory-area" :d="`${chartPath(memorySamples, 100)} L1000,100 L0,100 Z`"/>
          <path class="chart-line memory-line" :d="chartPath(memorySamples, 100)"/>
        </svg>
        <div class="chart-axis"><span>−4.5 MIN</span><span>−2 MIN</span><span>NOW</span></div>
      </div>
    </div>
    <DataTable
      class="memory-process-console"
      :columns="columns"
      :items="memoryProcesses"
      :empty="!memoryProcessesAvailable || !memoryProcesses.length"
      section-title="PROCESS RESIDENT MEMORY"
      section-subtitle="RSS / WORKING SET · TOP 20"
      :key-prop="(process) => process.pid"
    >
      <template #empty>
        <p v-if="!memoryProcessesAvailable" class="process-empty">PROCESS MEMORY SAMPLE UNAVAILABLE</p>
        <p v-else class="process-empty">NO PROCESS MEMORY READINGS AVAILABLE</p>
      </template>
      <template #row="{ item: process, index }">
        <span class="detail-process-identity">
          <i>{{ String(index + 1).padStart(2, "0") }}</i>
          <b :title="process.name">{{ process.name }}</b>
        </span>
        <span class="process-pid">{{ process.pid }}</span>
        <span class="detail-process-reading">
          <b>{{ formatBytes(process.memory_bytes) }}</b>
          <i :style="{ width: processMetricWidth(process.memory_bytes, largestMemoryProcess) }"></i>
        </span>
      </template>
    </DataTable>
    <div class="panel-status">
      <span class="calibration">✥</span> PHYSICAL MEMORY <i>·</i> {{ usedMemoryPercent.toFixed(0) }}% COMMITTED <i>·</i> LOCAL SAMPLING
    </div>
  </section>
</template>
