<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { KarenCli, type CliEvent } from "./cli";
import TabWheel from "./components/TabWheel.vue";
import UpdatePrompt from "./components/UpdatePrompt.vue";
import { formatTraySummary, formatTrayTitle, type TabId, type TabItem } from "./components/tabWheel";
import { formatBytes } from "./format";
import type { TreeTile } from "./treemap";
import type {
  BatteryInfo,
  Crumb,
  DiskMark,
  Finding,
  MemoryProcess,
  NetworkInfo,
  PowerPreset,
  RemovalState,
  Sibling,
  TreeMode,
  Volume,
  CpuProcess,
} from "./types";
import BatteryView from "./views/BatteryView.vue";
import CpuView from "./views/CpuView.vue";
import DiskTreeView from "./views/DiskTreeView.vue";
import MemoryView from "./views/MemoryView.vue";
import NetworkView from "./views/NetworkView.vue";
import RuntimeView from "./views/RuntimeView.vue";
import { useI18n, type SupportedLocale } from "./i18n";

const { t, text, locale, supportedLocales, setLocale } = useI18n();

const status = ref("Starting monitor");
const isMacOS = navigator.userAgent.includes("Macintosh");
const tab = ref<TabId>("runtime");
const treeMode = ref<TreeMode>("size");
const includeHidden = ref(true);
const apparentSize = ref(false);
const treeDepth = ref(4);
const cpu = ref<number | null>(null);
const memoryUsed = ref<number | null>(null);
const memoryTotal = ref<number | null>(null);
const battery = ref<BatteryInfo>({ present: false });
const batterySamples = ref<number[]>([]);
const batteryPowerSamples = ref<number[]>([]);
const network = ref<NetworkInfo>({ total_rx_bytes: 0, total_tx_bytes: 0, interfaces: [] });
const networkRxSamples = ref<number[]>([]);
const networkTxSamples = ref<number[]>([]);
let unlistenTray: UnlistenFn | undefined;
const volumes = ref<Volume[]>([]);
const selectedVolume = ref("");
const treeRoot = ref("");
const tiles = ref<TreeTile[]>([]);
const files = ref(0);
const dirs = ref(0);
const bytes = ref(0);
const scanProgress = ref({ files: 0, dirs: 0, bytes: 0 });
const error = ref("");
const appVersion = ref("");
const scanning = ref(false);
const previewing = ref(false);
const cpuSamples = ref<number[]>([]);
const memorySamples = ref<number[]>([]);
const cpuProcesses = ref<CpuProcess[]>([]);
const cpuProcessesAvailable = ref(false);
const memoryProcesses = ref<MemoryProcess[]>([]);
const memoryProcessesAvailable = ref(false);
const breadcrumbs = ref<Crumb[]>([]);
const power = ref<PowerPreset>("balanced");
const workers = ref<number | null>(null);
const unreadable = ref(0);
const exportStatus = ref("");
const uiZoom = ref(1);
const removal = ref<RemovalState | null>(null);
const siblings = ref<Sibling[]>([]);
const findings = ref<Finding[]>([]);
const marks = ref<DiskMark[]>([]);
const selection = ref<Record<string, unknown> | null>(null);
const review = ref<Record<string, unknown> | null>(null);
const reviewScreen = ref(false);
const removalConfirm = ref(false);
const removalMode = ref<"trash" | "permanent">("trash");
const removalStatus = ref("");
let cli: KarenCli | undefined;
let refreshTimer: number | undefined;
let activeScanId: string | undefined;
let pendingVolumePath: string | undefined;

const usedMemoryPercent = computed(() => memoryTotal.value
  ? Math.min(100, ((memoryUsed.value ?? 0) / memoryTotal.value) * 100)
  : 0);

function syncTrayStatus(): void {
  if (!isTauri()) return;
  const tabLabel = (tabId: string) => t(`nav.${tabId}`);
  void invoke("update_tray_status", {
    labels: {
      title: formatTrayTitle(cpu.value, usedMemoryPercent.value),
      summary: formatTraySummary(cpu.value, usedMemoryPercent.value, battery.value.present, battery.value.percent),
      toggleWindow: text("Show / Hide Karen"),
      runtimeLabel: tabLabel("runtime"),
      cpuLabel: tabLabel("cpu"),
      memoryLabel: tabLabel("memory"),
      batteryLabel: tabLabel("battery"),
      networkLabel: tabLabel("network"),
      spaceLabel: tabLabel("space"),
      checkUpdateLabel: t("update.checkMenu"),
      quitLabel: text("Quit Karen"),
    },
  });
}

function receive(event: CliEvent): void {
  if (event.event === "ready") {
    status.value = "Live telemetry";
    void cli?.send({ id: crypto.randomUUID(), command: "get_settings" });
  }
  if (event.event === "settings") {
    if (typeof event.power === "string" && ["miser", "balanced", "aggressive", "drain"].includes(event.power)) {
      power.value = event.power as PowerPreset;
    }
    if (typeof event.ui_zoom === "number") applyUiZoom(event.ui_zoom);
  }
  if (event.event === "resources") {
    cpu.value = typeof event.cpu_percent === "number" ? event.cpu_percent : null;
    memoryUsed.value = typeof event.memory_used_bytes === "number" ? event.memory_used_bytes : null;
    memoryTotal.value = typeof event.memory_total_bytes === "number" ? event.memory_total_bytes : null;
    cpuProcessesAvailable.value = event.cpu_processes_available === true;
    cpuProcesses.value = readCpuProcesses(event.cpu_processes);
    memoryProcessesAvailable.value = event.memory_processes_available === true;
    memoryProcesses.value = readMemoryProcesses(event.memory_processes);
    if (cpu.value !== null) pushSample(cpuSamples.value, cpu.value);
    if (memoryTotal.value) pushSample(memorySamples.value, usedMemoryPercent.value);
    if (typeof event.battery === "object" && event.battery !== null) {
      battery.value = event.battery as BatteryInfo;
      if (typeof battery.value.percent === "number") pushSample(batterySamples.value, battery.value.percent);
      if (typeof battery.value.power_watts === "number") pushSample(batteryPowerSamples.value, battery.value.power_watts);
    }
    if (typeof event.network === "object" && event.network !== null) {
      network.value = event.network as NetworkInfo;
      if (typeof network.value.rx_bytes_per_sec === "number") pushSample(networkRxSamples.value, network.value.rx_bytes_per_sec);
      if (typeof network.value.tx_bytes_per_sec === "number") pushSample(networkTxSamples.value, network.value.tx_bytes_per_sec);
    }
    syncTrayStatus();
  }
  if (event.event === "volumes" && Array.isArray(event.volumes)) {
    volumes.value = event.volumes as Volume[];
    if (removal.value?.phase === "done" && removal.value.freeAfter === null) {
      removal.value.freeAfter = volumes.value.find((volume) => volume.mount_point === selectedVolume.value)?.available_bytes ?? null;
    }
    if (!selectedVolume.value) {
      const points = volumes.value.map((volume) => volume.mount_point);
      selectedVolume.value = points.includes("/") ? "/" : points[0] ?? "";
    }
  }
  if (event.event === "scan_started" || event.event === "scan_progress") {
    if (event.id !== activeScanId) return;
    if (typeof event.workers === "number") workers.value = event.workers;
    scanning.value = true;
    previewing.value = true;
    status.value = "Mapping disk";
    scanProgress.value = {
      files: Number(event.files ?? scanProgress.value.files),
      dirs: Number(event.dirs ?? scanProgress.value.dirs),
      bytes: Number(event.bytes ?? scanProgress.value.bytes),
    };
  }
  if (event.event === "scan_preview") {
    if (event.id !== activeScanId) return;
    previewing.value = true;
    scanProgress.value = {
      files: Number(event.files ?? scanProgress.value.files),
      dirs: Number(event.dirs ?? scanProgress.value.dirs),
      bytes: Number(event.bytes ?? scanProgress.value.bytes),
    };
  }
  if (event.event === "scan_complete") {
    if (event.id !== activeScanId) return;
    activeScanId = undefined;
    scanning.value = false;
    previewing.value = false;
    status.value = "Live telemetry";
    treeRoot.value = String(event.root ?? treeRoot.value);
    unreadable.value = Number(event.unreadable ?? 0);
    files.value = Number(event.files ?? 0);
    dirs.value = Number(event.dirs ?? 0);
    bytes.value = Number(event.bytes ?? 0);
    scanProgress.value = { files: files.value, dirs: dirs.value, bytes: bytes.value };
    tiles.value = Array.isArray(event.tiles) ? event.tiles as TreeTile[] : [];
    requestTreeView({ path: treeRoot.value });
    startPendingVolumeNavigation();
  }
  if (event.event === "tree_cache_miss") {
    const path = String(event.path ?? event.root ?? "");
    void scan(path);
  }
  if (event.event === "tree_view") {
    const keepView = keepViewIds.delete(String(event.id));
    treeRoot.value = String(event.root ?? treeRoot.value);
    if (!keepView) {
      files.value = Number(event.files ?? 0);
      dirs.value = Number(event.dirs ?? 0);
      bytes.value = Number(event.bytes ?? 0);
      tiles.value = Array.isArray(event.tiles) ? event.tiles as TreeTile[] : [];
      breadcrumbs.value = Array.isArray(event.breadcrumbs) ? event.breadcrumbs as Crumb[] : [];
      siblings.value = Array.isArray(event.siblings) ? event.siblings as Sibling[] : [];
      findings.value = Array.isArray(event.worth_a_look) ? event.worth_a_look as Finding[] : [];
    }
    marks.value = Array.isArray(event.marks) ? event.marks as DiskMark[] : marks.value;
    selection.value = typeof event.selection === "object" && event.selection !== null ? event.selection as Record<string, unknown> : null;
    if (typeof event.unreadable === "number") unreadable.value = event.unreadable;
  }
  if (event.event === "marks_export") {
    const exportText = String(event.text ?? "");
    void navigator.clipboard?.writeText(exportText).catch(() => undefined);
    exportStatus.value = event.saved_path ? `${text("SAVED")} ${String(event.saved_path)}` : `${text("COPIED")} ${text(event.format === "prompt" ? "AGENT PROMPT" : "PATH LIST")}`;
  }
  if (event.event === "directory_chosen" && typeof event.path === "string") void scan(event.path);
  if (event.event === "reveal_done") status.value = "Live telemetry";
  if (event.event === "mark_changed" || event.event === "marks_changed") {
    marks.value = Array.isArray(event.marks) ? event.marks as DiskMark[] : [];
    review.value = null;
    if (event.event === "mark_changed") {
      requestTreeView({ path: treeRoot.value, selectedPath: String(event.path ?? treeRoot.value), root: treeRoot.value, open: false });
    }
  }
  if (event.event === "review") {
    review.value = event;
    reviewScreen.value = true;
  }
  if (event.event === "removal_started") {
    removalStatus.value = text("REMOVAL IN PROGRESS");
    const selectedVol = volumes.value.find((v) => v.mount_point === selectedVolume.value);
    removal.value = {
      phase: "running", total: Number(review.value?.targets ?? 0), completed: 0, removed: 0, bytes: 0, failed: 0,
      errors: [], freeBefore: selectedVol?.available_bytes ?? null, freeAfter: null,
    };
  }
  if (event.event === "removal_progress" && removal.value) {
    removal.value.total = Number(event.total ?? removal.value.total);
    removal.value.completed = Number(event.completed ?? removal.value.completed);
  }
  if (event.event === "removal_item" && removal.value) {
    removal.value.completed += 1;
    if (event.ok === false) {
      removalStatus.value = `${text("FAILED")}: ${String(event.error ?? "removal error")}`;
      removal.value.errors.push(`${String(event.path)} · ${String(event.error ?? "removal error")}`);
    }
  }
  if (event.event === "removal_complete") {
    removalStatus.value = `${text("REMOVED")} ${String(event.removed)} · ${formatBytes(Number(event.bytes ?? 0))} · ${String(event.failed)} ${text("FAILED")}`;
    if (removal.value) {
      Object.assign(removal.value, { phase: "done", removed: Number(event.removed ?? 0), bytes: Number(event.bytes ?? 0), failed: Number(event.failed ?? 0) });
    }
    marks.value = [];
    review.value = null;
    removalConfirm.value = false;
    refresh();
  }
  if (event.event === "error") {
    keepViewIds.delete(String(event.id));
    if (event.id !== activeScanId) {
      if (activeScanId) return;
      status.value = "Monitor fault";
      error.value = String(event.message ?? event.code ?? "Unknown error");
      return;
    }
    const cancelledForSwitch = pendingVolumePath !== undefined;
    activeScanId = undefined;
    scanning.value = false;
    previewing.value = false;
    if (!cancelledForSwitch) {
      status.value = "Monitor fault";
      error.value = String(event.message ?? event.code ?? "Unknown error");
    }
    if (cancelledForSwitch) startPendingVolumeNavigation();
  }
}

function readCpuProcesses(value: unknown): CpuProcess[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((item): CpuProcess[] => {
    if (typeof item !== "object" || item === null) return [];
    const process = item as Record<string, unknown>;
    if (typeof process.pid !== "number" || !Number.isSafeInteger(process.pid)) return [];
    if (typeof process.name !== "string" || typeof process.cpu_percent !== "number") return [];
    if (!Number.isFinite(process.cpu_percent) || process.cpu_percent < 0) return [];
    return [{ pid: process.pid, name: process.name, cpu_percent: process.cpu_percent }];
  }).slice(0, 20);
}

function readMemoryProcesses(value: unknown): MemoryProcess[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((item): MemoryProcess[] => {
    if (typeof item !== "object" || item === null) return [];
    const process = item as Record<string, unknown>;
    if (typeof process.pid !== "number" || !Number.isSafeInteger(process.pid)) return [];
    if (typeof process.name !== "string" || typeof process.memory_bytes !== "number") return [];
    if (!Number.isSafeInteger(process.memory_bytes) || process.memory_bytes < 0) return [];
    return [{ pid: process.pid, name: process.name, memory_bytes: process.memory_bytes }];
  }).slice(0, 20);
}

function refresh(): void {
  void cli?.send({ id: crypto.randomUUID(), command: "resources" });
  void cli?.send({ id: crypto.randomUUID(), command: "volumes" });
}

async function scan(path = selectedVolume.value): Promise<void> {
  if (activeScanId || !path) return;
  error.value = "";
  keepViewIds.clear();
  scanProgress.value = { files: 0, dirs: 0, bytes: 0 };
  const id = crypto.randomUUID();
  activeScanId = id;
  previewing.value = true;
  scanning.value = true;
  tab.value = "disk";
  try {
    await cli?.send({
      id, command: "scan", root: path,
      include_hidden: includeHidden.value, apparent_size: apparentSize.value, power: power.value,
    });
  } catch (cause) {
    activeScanId = undefined;
    scanning.value = false;
    error.value = String(cause);
  }
}

function selectVolume(path: string): void {
  tab.value = "disk";
  if (path === selectedVolume.value) {
    if (treeRoot.value !== path || tiles.value.length === 0) {
      requestTreeView({ path, selectedPath: path, root: path, open: true });
    }
    return;
  }
  selectedVolume.value = path;
  if (activeScanId) {
    pendingVolumePath = path;
    void cli?.send({ id: crypto.randomUUID(), command: "cancel_scan" });
    return;
  }
  requestTreeView({ path, selectedPath: path, root: path, open: true });
}

function startPendingVolumeNavigation(): void {
  const path = pendingVolumePath;
  if (!path || activeScanId) return;
  pendingVolumePath = undefined;
  requestTreeView({ path, selectedPath: path, root: path });
}

function openDiskTree(): void {
  if (reviewScreen.value) cancelReview();
  tab.value = "disk";
  if (!scanning.value && tiles.value.length === 0 && selectedVolume.value) {
    requestTreeView({ path: treeRoot.value || selectedVolume.value });
  }
}

// Ids of select-only requests. The CLI answers them with the clicked node's parent as the view,
// which would read as drilling in, so only the selection of those replies is applied.
const keepViewIds = new Set<string>();

function requestTreeView(params: {
  path: string;
  selectedPath?: string;
  root?: string;
  open?: boolean;
  keepView?: boolean;
  metric?: "size" | "files";
  depth?: number;
  width?: number;
  height?: number;
}): void {
  const root = params.root || treeRoot.value || selectedVolume.value;
  const path = params.path;
  const selectedPath = params.selectedPath ?? path;
  const id = crypto.randomUUID();
  if (params.keepView) keepViewIds.add(id);
  void cli?.send({
    id,
    command: "tree_view",
    root,
    path: selectedPath,
    open: params.open ?? true,
    metric: params.metric ?? (treeMode.value === "files" ? "files" : "size"),
    depth: params.depth ?? treeDepth.value,
    width: params.width ?? 1200,
    height: params.height ?? 680,
  });
}

function toggleMark(path: string): void {
  void cli?.send({ id: crypto.randomUUID(), command: "toggle_mark", root: treeRoot.value, path });
}

function requestReview(): void {
  void cli?.send({ id: crypto.randomUUID(), command: "review", root: treeRoot.value });
}

function finishRemoval(): void {
  removal.value = null;
  reviewScreen.value = false;
  if (treeRoot.value) void scan(treeRoot.value);
}

function exportMarks(format: "paths" | "prompt", save: boolean): void {
  exportStatus.value = "";
  void cli?.send({ id: crypto.randomUUID(), command: "export_marks", root: treeRoot.value, format, save });
}

function clearMarks(): void {
  void cli?.send({ id: crypto.randomUUID(), command: "clear_marks", root: treeRoot.value });
  void cli?.send({ id: crypto.randomUUID(), command: "review", root: treeRoot.value });
}

function revealPath(path: string | null): void {
  if (path) void cli?.send({ id: crypto.randomUUID(), command: "reveal", path });
}

function chooseDirectory(): void {
  void cli?.send({ id: crypto.randomUUID(), command: "choose_directory" });
}

function openFullDiskAccess(): void {
  void cli?.send({ id: crypto.randomUUID(), command: "open_full_disk_access" });
}

function setPower(preset: PowerPreset): void {
  power.value = preset;
  void cli?.send({ id: crypto.randomUUID(), command: "set_settings", power: preset });
}

function cancelReview(): void {
  review.value = null;
  reviewScreen.value = false;
  removalConfirm.value = false;
  void cli?.send({ id: crypto.randomUUID(), command: "cancel_review" });
}

function confirmRemoval(mode: "trash" | "permanent"): void {
  removalMode.value = mode;
  void cli?.send({ id: crypto.randomUUID(), command: "confirm_remove", mode });
  removalConfirm.value = false;
}

function applyUiZoom(next: number): void {
  uiZoom.value = Math.max(0.6, Math.min(2, Math.round(next * 100) / 100));
  document.documentElement.style.setProperty("zoom", String(uiZoom.value));
}

function setUiZoom(next: number): void {
  applyUiZoom(next);
  void cli?.send({ id: crypto.randomUUID(), command: "set_settings", ui_zoom: uiZoom.value });
}

function handleGlobalKeys(event: KeyboardEvent): void {
  const command = event.metaKey || event.ctrlKey;
  if (command && ["=", "+"].includes(event.key)) { event.preventDefault(); setUiZoom(uiZoom.value + 0.1); }
  else if (command && event.key === "-") { event.preventDefault(); setUiZoom(uiZoom.value - 0.1); }
  else if (command && event.key === "0") { event.preventDefault(); setUiZoom(1); }
}

// Maximize on the second press: a native drag begun by the first press can swallow `dblclick` on macOS.
function pressTitleBar(event: MouseEvent): void {
  if (event.button !== 0) return;
  if (event.target instanceof Element && event.target.closest("button, select, label")) return;
  if (event.detail === 2) toggleMaximizeWindow();
  else if (event.detail < 3) void getCurrentWindow().startDragging();
}

function minimizeWindow(): void {
  void getCurrentWindow().minimize();
}

function toggleMaximizeWindow(): void {
  void getCurrentWindow().toggleMaximize();
}

function closeWindow(): void {
  void getCurrentWindow().close();
}

function onLocaleChange(event: Event): void {
  const target = event.target as HTMLSelectElement;
  setLocale(target.value as SupportedLocale);
  syncTrayStatus();
}

const tabItems = computed<TabItem[]>(() => [
  { id: "runtime", index: "01", label: t("nav.runtime"), badge: cpu.value === null ? text("WAIT") : text("LIVE") },
  { id: "cpu", index: "02", label: t("nav.cpu"), badge: cpuProcessesAvailable.value ? text("LIVE") : text("WAIT") },
  { id: "memory", index: "03", label: t("nav.memory"), badge: memoryTotal.value === null ? text("WAIT") : text("LIVE") },
  { id: "battery", index: "04", label: t("nav.battery"), badge: battery.value.present ? `${battery.value.percent ?? "—"}%` : text("AC") },
  { id: "network", index: "05", label: t("nav.network"), badge: network.value.rx_bytes_per_sec == null ? text("WAIT") : text("LIVE") },
  { id: "disk", index: "06", label: t("nav.space"), badge: scanning.value ? text("MAP") : treeRoot.value ? text("READY") : text("OPEN") },
]);

function selectTab(target: TabId): void {
  if (target === "disk") openDiskTree();
  else tab.value = target;
}

function pushSample(samples: number[], value: number): void {
  samples.push(value);
  if (samples.length > 90) samples.shift();
}

function selectTrayTab(target: string): void {
  if (target === "space") selectTab("disk");
  else if (["runtime", "cpu", "memory", "battery", "network"].includes(target)) {
    selectTab(target as TabId);
  }
}

onMounted(async () => {
  window.addEventListener("keydown", handleGlobalKeys);
  if (!isTauri()) {
    status.value = "Open Karen.app to connect the CLI";
    return;
  }
  syncTrayStatus();
  // Display only: a failure must not hold up the CLI start below.
  void getVersion().then((version) => { appVersion.value = version; }, () => undefined);
  unlistenTray = await listen<string>("tray-navigate", ({ payload }) => selectTrayTab(payload));
  cli = new KarenCli(receive);
  try {
    await cli.start();
    refresh();
    refreshTimer = window.setInterval(refresh, 3000);
  } catch (cause) {
    status.value = "CLI unavailable";
    error.value = String(cause);
  }
});

onBeforeUnmount(() => {
  unlistenTray?.();
  window.removeEventListener("keydown", handleGlobalKeys);
  if (refreshTimer !== undefined) window.clearInterval(refreshTimer);
  void cli?.stop();
});
</script>

<template>
  <main class="instrument" :class="{ macos: isMacOS }">
    <header class="masthead" @mousedown="pressTitleBar">
      <div class="brand-lockup">
        <svg class="brand-seal" viewBox="0 0 512 512" aria-hidden="true">
          <rect x="0" y="0" width="512" height="512" rx="112" fill="#181510"/>
          <rect x="48" y="68" width="416" height="376" rx="60" fill="#C9A24D"/>
          <circle cx="96" cy="116" r="15" fill="#181510"/>
          <circle cx="416" cy="116" r="15" fill="#181510"/>
          <circle cx="96" cy="396" r="15" fill="#181510"/>
          <circle cx="416" cy="396" r="15" fill="#181510"/>
          <path d="M 256 368 L 84 196 A 243 243 0 0 1 428 196 Z" fill="#101416"/>
          <path d="M 332 136 A 216 216 0 0 1 398 188" fill="none" stroke="#C4703F" stroke-width="22" stroke-linecap="round"/>
          <path d="M 128 202 L 156 230 M 184 156 L 200 188 M 256 140 L 256 176 M 378 206 L 350 234" stroke="#F0E4C6" stroke-width="20" stroke-linecap="round" fill="none"/>
          <path d="M 256 368 L 318 162" stroke="#EFCB70" stroke-width="28" stroke-linecap="round"/>
          <polygon points="324,146 278,368 234,368" fill="#EFCB70"/>
          <circle cx="256" cy="368" r="46" fill="#C9A24D"/>
          <circle cx="256" cy="368" r="22" fill="#181510"/>
          <circle cx="256" cy="368" r="10" fill="#F0E4C6"/>
        </svg>
        <span class="brand-name">KAREN<span v-if="locale === 'zh-CN'"> · 凯衡</span><span v-else-if="locale === 'zh-TW'"> · 凱衡</span></span>
        <span class="brand-edition">{{ t("masthead.edition") }} · {{ text("SYSTEM MONITOR") }}</span>
        <span v-if="appVersion" class="brand-version">v{{ appVersion }}</span>
      </div>
      <div class="masthead-right">
        <label class="lang-picker">
          <select :value="locale" :aria-label="text('Select language')" @change="onLocaleChange">
            <option v-for="item in supportedLocales" :key="item.code" :value="item.code">
              {{ item.nativeName }}
            </option>
          </select>
        </label>
        <div class="telemetry-state" :class="{ fault: status.includes('fault') || status.includes('unavailable') }">
          <span class="state-lamp"></span><span>{{ text(status) }}</span><span class="state-divider">/</span><span>{{ text("LOCAL") }}</span>
        </div>
        <div class="window-controls" :aria-label="text('Window controls')">
          <button :aria-label="text('Minimize window')" :title="text('Minimize')" @mousedown.stop @click.stop="minimizeWindow"><i class="minimize-glyph"></i></button>
          <button :aria-label="text('Toggle window size')" :title="text('Toggle size')" @mousedown.stop @click.stop="toggleMaximizeWindow"><i class="maximize-glyph"></i></button>
          <button class="close-control" :aria-label="text('Close window')" :title="text('Close')" @mousedown.stop @click.stop="closeWindow"><i class="close-glyph"></i></button>
        </div>
      </div>
    </header>

    <TabWheel :tabs="tabItems" :active="tab" @select="selectTab" />

    <RuntimeView
      v-if="tab === 'runtime'"
      :cpu="cpu"
      :cpu-samples="cpuSamples"
      :cpu-processes="cpuProcesses"
      :cpu-processes-available="cpuProcessesAvailable"
      :memory-used="memoryUsed"
      :memory-total="memoryTotal"
      :memory-processes="memoryProcesses"
      :memory-processes-available="memoryProcessesAvailable"
      :battery="battery"
      :network="network"
      :volumes="volumes"
      @navigate="tab = $event"
      @select-volume="selectVolume"
    />

    <CpuView
      v-else-if="tab === 'cpu'"
      :cpu="cpu"
      :cpu-processes="cpuProcesses"
      :cpu-processes-available="cpuProcessesAvailable"
    />

    <MemoryView
      v-else-if="tab === 'memory'"
      :memory-used="memoryUsed"
      :memory-total="memoryTotal"
      :memory-samples="memorySamples"
      :memory-processes="memoryProcesses"
      :memory-processes-available="memoryProcessesAvailable"
    />

    <BatteryView
      v-else-if="tab === 'battery'"
      :battery="battery"
      :battery-samples="batterySamples"
      :battery-power-samples="batteryPowerSamples"
    />

    <NetworkView
      v-else-if="tab === 'network'"
      :network="network"
      :network-rx-samples="networkRxSamples"
      :network-tx-samples="networkTxSamples"
    />

    <DiskTreeView
      v-else
      v-model:tree-mode="treeMode"
      v-model:tree-depth="treeDepth"
      v-model:include-hidden="includeHidden"
      v-model:apparent-size="apparentSize"
      v-model:review-screen="reviewScreen"
      v-model:removal-confirm="removalConfirm"
      v-model:removal-mode="removalMode"
      :selected-volume="selectedVolume"
      :volumes="volumes"
      :tree-root="treeRoot"
      :tiles="tiles"
      :files="files"
      :dirs="dirs"
      :bytes="bytes"
      :scan-progress="scanProgress"
      :error="error"
      :scanning="scanning"
      :previewing="previewing"
      :workers="workers"
      :power="power"
      :unreadable="unreadable"
      :export-status="exportStatus"
      :removal="removal"
      :siblings="siblings"
      :findings="findings"
      :marks="marks"
      :selection="selection"
      :review="review"
      :breadcrumbs="breadcrumbs"
      :removal-status="removalStatus"
      :is-mac-o-s="isMacOS"
      @scan="scan"
      @select-volume="selectVolume"
      @request-tree-view="requestTreeView"
      @toggle-mark="toggleMark"
      @request-review="requestReview"
      @cancel-review="cancelReview"
      @confirm-removal="confirmRemoval"
      @finish-removal="finishRemoval"
      @export-marks="exportMarks"
      @clear-marks="clearMarks"
      @reveal-path="revealPath"
      @choose-directory="chooseDirectory"
      @open-full-disk-access="openFullDiskAccess"
      @set-power="setPower"
      @close-window="closeWindow"
    />
    <UpdatePrompt />
  </main>
</template>
