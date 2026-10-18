<script setup lang="ts">
import { chartPath, formatDuration } from "../format";
import { useI18n } from "../i18n";
import type { BatteryInfo } from "../types";

const { t } = useI18n();

defineProps<{
  battery: BatteryInfo;
  batterySamples: number[];
  batteryPowerSamples: number[];
}>();
</script>

<template>
  <section class="view metric-view battery-view">
    <div class="view-heading">
      <h1>{{ t("nav.battery") }}</h1>
      <span class="volume-count">
        {{ battery.present ? `${(battery.state ?? "unknown").toUpperCase()} · ${battery.percent ?? "—"}%` : t("battery.acPower") }}
      </span>
    </div>
    <div class="metric-console memory-console battery-console">
      <div class="metric-main">
        <span class="metric-caption">CHARGE RESERVE</span>
        <strong class="metric-value memory-value">{{ battery.present ? `${battery.percent ?? "—"}%` : "AC" }}</strong>
        <div class="metric-ruler"><i :style="{ width: `${battery.present ? (battery.percent ?? 0) : 100}%` }"></i></div>
        <div class="metric-state">
          <span>SUPPLY STATE</span>
          <b>{{ battery.present ? (battery.state ?? "unknown").toUpperCase() : "NO BATTERY DETECTED (AC POWER)" }}</b>
        </div>
        <div class="metric-state">
          <span>POWER RATE</span>
          <b>{{ battery.power_watts != null ? `${battery.power_watts.toFixed(1)} W` : "—" }}</b>
        </div>
        <div class="metric-state">
          <span>ESTIMATED RUNTIME</span>
          <b>{{ formatDuration(battery.time_to_empty_secs ?? battery.time_to_full_secs) }}</b>
        </div>
      </div>
      <div class="chart-console">
        <div class="chart-heading">
          <span>CHARGE &amp; WATTAGE TRACE</span><span>3 SECOND SAMPLE <i>/</i> 4.5 MIN WINDOW</span>
        </div>
        <svg class="history-chart" viewBox="0 0 1000 100" preserveAspectRatio="none" role="img" aria-label="Recent battery charge history">
          <path class="chart-grid" d="M0 1H1000 M0 50H1000 M0 99H1000"/>
          <path v-if="batterySamples.length > 1" class="chart-area" :d="`${chartPath(batterySamples, 100)} L1000,100 L0,100 Z`"/>
          <path class="chart-line cpu-line" :d="chartPath(batterySamples, 100)"/>
          <path class="chart-line memory-line" :d="chartPath(batteryPowerSamples, 100)"/>
        </svg>
        <div class="chart-axis"><span>−4.5 MIN</span><span>−2 MIN</span><span>NOW</span></div>
      </div>
    </div>
    <div class="detail-process-console memory-process-console battery-ledger">
      <div class="instrument-section-heading">
        <span>CELL CAPACITY &amp; ELECTRICAL TELEMETRY</span>
        <span>{{ battery.present ? "SMART BATTERY" : "DESKTOP / AC LINE" }}</span>
      </div>
      <p v-if="!battery.present" class="process-empty">No battery detected (AC power) — running on external mains supply.</p>
      <div v-else class="battery-fact-grid">
        <div class="battery-fact"><span>CURRENT ENERGY</span><b>{{ battery.energy_wh != null ? `${battery.energy_wh.toFixed(1)} Wh` : "—" }}</b></div>
        <div class="battery-fact"><span>FULL CHARGE CAPACITY</span><b>{{ battery.energy_full_wh != null ? `${battery.energy_full_wh.toFixed(1)} Wh` : "—" }}</b></div>
        <div class="battery-fact"><span>FACTORY DESIGN CAPACITY</span><b>{{ battery.energy_design_wh != null ? `${battery.energy_design_wh.toFixed(1)} Wh` : "—" }}</b></div>
        <div class="battery-fact"><span>CELL HEALTH</span><b>{{ battery.health_percent != null ? `${battery.health_percent}%` : "—" }}</b></div>
        <div class="battery-fact"><span>CYCLE COUNT</span><b>{{ battery.cycle_count ?? "—" }}</b></div>
        <div class="battery-fact"><span>PACK VOLTAGE</span><b>{{ battery.voltage_volts != null ? `${battery.voltage_volts.toFixed(2)} V` : "—" }}</b></div>
        <div class="battery-fact"><span>PACK TEMPERATURE</span><b>{{ battery.temperature_celsius != null ? `${battery.temperature_celsius.toFixed(1)} °C` : "—" }}</b></div>
      </div>
    </div>
    <div class="panel-status">
      <span class="calibration">✥</span> POWER TELEMETRY <i>·</i> {{ battery.present ? `HEALTH ${battery.health_percent ?? "—"}%` : "AC MAINS" }} <i>·</i> LOCAL SAMPLING
    </div>
  </section>
</template>
