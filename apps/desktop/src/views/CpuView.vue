<script setup lang="ts">
import { computed } from "vue";
import DataTable from "../components/DataTable.vue";
import { processMetricWidth } from "../format";
import { useI18n } from "../i18n";
import type { CpuProcess } from "../types";

const { t, text, decimal } = useI18n();

const props = defineProps<{
  cpu: number | null;
  cpuProcesses: CpuProcess[];
  cpuProcessesAvailable: boolean;
}>();

const largestCpuProcess = computed(() => props.cpuProcesses[0]?.cpu_percent ?? 0);
const columns = computed(() => [t("runtime.processes"), t("network.colPid"), t("cpu.avgUtilization")]);
</script>

<template>
  <section class="view metric-view detail-view">
    <div class="view-heading">
      <h1>{{ t("nav.cpu") }}</h1>
      <span class="volume-count">{{ text("SYSTEM") }} {{ cpu ?? "—" }}% · {{ text("100% = ONE CORE") }}</span>
    </div>
    <DataTable
      :columns="columns"
      :items="cpuProcesses"
      :empty="!cpuProcessesAvailable || !cpuProcesses.length"
      :key-prop="(process) => process.pid"
    >
      <template #empty>
        <p v-if="!cpuProcessesAvailable" class="process-empty">{{ text("SAMPLING BASELINE — PROCESS LOAD WARMING UP") }}</p>
        <p v-else class="process-empty">{{ text("NO PROCESS CPU READINGS AVAILABLE") }}</p>
      </template>
      <template #row="{ item: process, index }">
        <span class="detail-process-identity">
          <i>{{ String(index + 1).padStart(2, "0") }}</i>
          <b :title="process.name">{{ process.name }}</b>
        </span>
        <span class="process-pid">{{ process.pid }}</span>
        <span class="detail-process-reading">
          <b>{{ decimal(process.cpu_percent) }}%</b>
          <i :style="{ width: processMetricWidth(process.cpu_percent, largestCpuProcess) }"></i>
        </span>
      </template>
    </DataTable>
    <div class="panel-status">
      <span class="calibration">✥</span> {{ text("PROCESS CPU") }} <i>·</i> {{ text("DESCENDING LOAD") }} <i>·</i> {{ text("TOP 20") }} <i>·</i> {{ text("3 SECOND SAMPLE") }}
    </div>
  </section>
</template>
