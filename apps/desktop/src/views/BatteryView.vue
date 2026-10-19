<script setup lang="ts">
import { chartPath, formatDuration } from "../format";
import { useI18n } from "../i18n";
import type { BatteryInfo } from "../types";

const { t, text, decimal, batteryState } = useI18n();

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
        {{ battery.present ? `${batteryState(battery.state)} · ${battery.percent ?? "—"}%` : t("battery.acPower") }}
      </span>
    </div>
    <div class="metric-console memory-console battery-console">
      <div class="metric-main">
        <span class="metric-caption">{{ text("CHARGE RESERVE") }}</span>
        <strong class="metric-value memory-value">{{ battery.present ? `${battery.percent ?? "—"}%` : text("AC") }}</strong>
        <div class="metric-ruler"><i :style="{ width: `${battery.present ? (battery.percent ?? 0) : 100}%` }"></i></div>
        <div class="metric-state">
          <span>{{ text("SUPPLY STATE") }}</span>
          <b>{{ battery.present ? batteryState(battery.state) : text("NO BATTERY DETECTED (AC POWER)") }}</b>
        </div>
        <div class="metric-state">
          <span>{{ text("POWER RATE") }}</span>
          <b>{{ battery.power_watts != null ? `${decimal(battery.power_watts)} W` : "—" }}</b>
        </div>
        <div class="metric-state">
          <span>{{ text("ESTIMATED RUNTIME") }}</span>
          <b>{{ formatDuration(battery.time_to_empty_secs ?? battery.time_to_full_secs) }}</b>
        </div>
      </div>
      <div class="chart-console">
        <div class="chart-heading">
          <span>{{ text("CHARGE & WATTAGE TRACE") }}</span><span>{{ text("3 SECOND SAMPLE") }} <i>/</i> {{ text("4.5 MIN WINDOW") }}</span>
        </div>
        <svg class="history-chart" viewBox="0 0 1000 100" preserveAspectRatio="none" role="img" :aria-label="text('Recent battery charge history')">
          <path class="chart-grid" d="M0 1H1000 M0 50H1000 M0 99H1000"/>
          <path v-if="batterySamples.length > 1" class="chart-area" :d="`${chartPath(batterySamples, 100)} L1000,100 L0,100 Z`"/>
          <path class="chart-line cpu-line" :d="chartPath(batterySamples, 100)"/>
          <path class="chart-line memory-line" :d="chartPath(batteryPowerSamples, 100)"/>
        </svg>
        <div class="chart-axis"><span>{{ text("−4.5 MIN") }}</span><span>{{ text("−2 MIN") }}</span><span>{{ text("NOW") }}</span></div>
      </div>
    </div>
    <div class="detail-process-console memory-process-console battery-ledger">
      <div class="instrument-section-heading">
        <span>{{ text("CELL CAPACITY & ELECTRICAL TELEMETRY") }}</span>
        <span>{{ battery.present ? text("SMART BATTERY") : text("DESKTOP / AC LINE") }}</span>
      </div>
      <p v-if="!battery.present" class="process-empty">{{ text("No battery detected (AC power) — running on external mains supply.") }}</p>
      <div v-else class="battery-fact-grid">
        <div class="battery-fact"><span>{{ text("CURRENT ENERGY") }}</span><b>{{ battery.energy_wh != null ? `${decimal(battery.energy_wh)} Wh` : "—" }}</b></div>
        <div class="battery-fact"><span>{{ text("FULL CHARGE CAPACITY") }}</span><b>{{ battery.energy_full_wh != null ? `${decimal(battery.energy_full_wh)} Wh` : "—" }}</b></div>
        <div class="battery-fact"><span>{{ text("FACTORY DESIGN CAPACITY") }}</span><b>{{ battery.energy_design_wh != null ? `${decimal(battery.energy_design_wh)} Wh` : "—" }}</b></div>
        <div class="battery-fact"><span>{{ text("CELL HEALTH") }}</span><b>{{ battery.health_percent != null ? `${battery.health_percent}%` : "—" }}</b></div>
        <div class="battery-fact"><span>{{ text("CYCLE COUNT") }}</span><b>{{ battery.cycle_count ?? "—" }}</b></div>
        <div class="battery-fact"><span>{{ text("PACK VOLTAGE") }}</span><b>{{ battery.voltage_volts != null ? `${decimal(battery.voltage_volts, 2)} V` : "—" }}</b></div>
        <div class="battery-fact"><span>{{ text("PACK TEMPERATURE") }}</span><b>{{ battery.temperature_celsius != null ? `${decimal(battery.temperature_celsius)} °C` : "—" }}</b></div>
      </div>
    </div>
    <div class="panel-status">
      <span class="calibration">✥</span> {{ text("POWER TELEMETRY") }} <i>·</i> {{ battery.present ? `${text("HEALTH")} ${battery.health_percent ?? "—"}%` : text("AC MAINS") }} <i>·</i> {{ text("LOCAL SAMPLING") }}
    </div>
  </section>
</template>
