<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { KarenCli, type CliEvent } from "./cli";
import TabWheel from "./TabWheel.vue";
import { formatTraySummary, formatTrayTitle, type TabId, type TabItem } from "./tabWheel";
import { categoryColor, drawTreemap, tileAt, type TreeTile } from "./treemap";

type Volume = {
  mount_point: string;
  device?: string | null;
  total_bytes?: number | null;
  free_bytes?: number | null;
  available_bytes?: number | null;
};

type CpuProcess = {
  pid: number;
  name: string;
  cpu_percent: number;
};
type MemoryProcess = { pid: number; name: string; memory_bytes: number };
type BatteryInfo = {
  present: boolean;
  state?: string | null;
  percent?: number | null;
  power_watts?: number | null;
  energy_wh?: number | null;
  energy_full_wh?: number | null;
  energy_design_wh?: number | null;
  health_percent?: number | null;
  voltage_volts?: number | null;
  temperature_celsius?: number | null;
  cycle_count?: number | null;
  time_to_empty_secs?: number | null;
  time_to_full_secs?: number | null;
};
type NetworkInterface = {
  name: string;
  rx_bytes_per_sec?: number | null;
  tx_bytes_per_sec?: number | null;
  total_rx_bytes: number;
  total_tx_bytes: number;
  packets_rx: number;
  packets_tx: number;
  errors_rx: number;
  errors_tx: number;
};
type NetworkInfo = {
  rx_bytes_per_sec?: number | null;
  tx_bytes_per_sec?: number | null;
  total_rx_bytes: number;
  total_tx_bytes: number;
  interfaces: NetworkInterface[];
};
type Sibling = { name: string; path: string; bytes: number; is_dir: boolean; category?: string };
type Crumb = { path: string; label: string; siblings?: Sibling[] };
type DiskMark = { path: string; bytes: number; is_dir: boolean; hidden: boolean };
type Finding = { path: string; bytes: number; reason: string };
type BlockedMark = { path: string; reason: string };

const status = ref("Starting monitor");
const isMacOS = navigator.userAgent.includes("Macintosh");
const tab = ref<"runtime" | "cpu" | "memory" | "battery" | "network" | "disk">("runtime");
const treeMode = ref<"size" | "files" | "age">("size");
const treeModes = ["size", "files", "age"] as const;
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
const treemapCanvas = ref<HTMLCanvasElement | null>(null);
const treemapSize = ref({ width: 1200, height: 680 });
const files = ref(0);
const dirs = ref(0);
const bytes = ref(0);
const scanProgress = ref({ files: 0, dirs: 0, bytes: 0 });
const error = ref("");
const scanning = ref(false);
const previewing = ref(false);
const volumeMenuOpen = ref(false);
const volumePickerTrigger = ref<HTMLButtonElement | null>(null);
const volumeMenuStyle = ref<Record<string, string>>({});
const volumeMenuAbove = ref(false);
const cpuSamples = ref<number[]>([]);
const memorySamples = ref<number[]>([]);
const cpuProcesses = ref<CpuProcess[]>([]);
const cpuProcessesAvailable = ref(false);
const memoryProcesses = ref<MemoryProcess[]>([]);
const memoryProcessesAvailable = ref(false);
const breadcrumbs = ref<Crumb[]>([]);
type PowerPreset = "miser" | "balanced" | "aggressive" | "drain";
const powerPresets: { id: PowerPreset; label: string; note: string }[] = [
  { id: "miser", label: "Miser", note: "fewest workers, lowest battery use" },
  { id: "balanced", label: "Balanced", note: "default scan speed" },
  { id: "aggressive", label: "Aggressive", note: "more workers, faster scan" },
  { id: "drain", label: "Drain My Battery", note: "every available worker" },
];
const power = ref<PowerPreset>("balanced");
const workers = ref<number | null>(null);
const powerMenuOpen = ref(false);
const history = ref<{ root: string; path: string }[]>([]);
const historyIndex = ref(-1);
let replayingHistory = false;
let lastViewDir = "";
const filterOpen = ref(false);
const filterText = ref("");
const filterInput = ref<HTMLInputElement | null>(null);
const zoom = ref({ scale: 1, x: 0, y: 0 });
const helpOpen = ref(false);
const crumbMenu = ref<number | null>(null);
const unreadable = ref(0);
const exportStatus = ref("");
const uiZoom = ref(1);
const removal = ref<{
  phase: "running" | "done"; total: number; completed: number; removed: number; bytes: number;
  failed: number; errors: string[]; freeBefore: number | null; freeAfter: number | null;
} | null>(null);
let pointerLast = false;
const inspectorOpen = ref(true);
const inspectorWidth = ref(286);
const viewportWidth = ref(window.innerWidth);
const showInspector = computed(() => inspectorOpen.value && viewportWidth.value >= 832);
const siblings = ref<Sibling[]>([]);
const findings = ref<Finding[]>([]);
const marks = ref<DiskMark[]>([]);
const selection = ref<Record<string, unknown> | null>(null);
const hoveredTile = ref<{ tile: TreeTile; x: number; y: number } | null>(null);
const review = ref<Record<string, unknown> | null>(null);
const reviewScreen = ref(false);
const reviewMarks = computed(() => Array.isArray(review.value?.marks)
  ? review.value.marks as DiskMark[]
  : marks.value);
const blockedMarks = computed(() => Array.isArray(review.value?.blocked)
  ? review.value.blocked as BlockedMark[]
  : []);
const removalConfirm = ref(false);
const removalMode = ref<"trash" | "permanent">("trash");
const removalStatus = ref("");
let cli: KarenCli | undefined;
let refreshTimer: number | undefined;
let activeScanId: string | undefined;
let pendingVolumePath: string | undefined;
let treemapResizeObserver: ResizeObserver | undefined;

const selected = computed(() => volumes.value.find((volume) => volume.mount_point === selectedVolume.value));
const visibleCrumbs = computed(() => {
  const all = breadcrumbs.value.map((crumb, index) => ({ crumb, index }));
  if (all.length <= 5) return all.map((item) => ({ ...item, gap: false }));
  return [{ ...all[0], gap: false }, { crumb: all[0].crumb, index: -1, gap: true }, ...all.slice(-3).map((item) => ({ ...item, gap: false }))];
});
const workerLabel = computed(() => workers.value === null ? "" : ` · ${workers.value} WORKERS`);
const powerLabel = computed(() => powerPresets.find((preset) => preset.id === power.value)?.label ?? "Balanced");
const removalFreeDelta = computed(() => removal.value?.freeBefore != null && removal.value.freeAfter != null
  ? removal.value.freeAfter - removal.value.freeBefore
  : null);
const helpRows: [string, string][] = [
  ["Click / double-click tile", "Select / open directory (click a selected directory opens it)"],
  ["Right-click tile", "Select and reveal in the file manager"],
  ["Ctrl/Cmd-click, middle-click", "Toggle mark without moving selection"],
  ["Enter", "Open selected or pointed directory"],
  ["Space, X", "Toggle mark"],
  ["Arrows, H J K L", "Move selection between tiles"],
  ["Tab, Shift-Tab", "Next / previous sibling"],
  ["Backspace, U", "Parent directory"],
  ["Esc", "Clear filter, then go up one level"],
  ["Scroll, Shift-scroll", "Zoom toward pointer, pan when magnified"],
  ["- = + 0", "Shrink, magnify, reset treemap zoom"],
  ["[ ]", "Fewer / more tree levels"],
  ["Cmd/Ctrl + - 0", "Interface zoom"],
  ["Alt-Left/Right, Cmd-[ ]", "History back / forward"],
  ["/ or S", "Filter by name"],
  ["T", "Cycle Size, Files, Age"],
  ["D, I", "Toggle apparent size, hidden files (rescans)"],
  ["R", "Rescan current root"],
  ["Cmd/Ctrl-O", "Choose a directory to scan"],
  ["G", "Widen to whole volume"],
  ["P", "Toggle inspection panel"],
  ["O", "Reveal in file manager"],
  ["C", "Review marked paths"],
  ["?", "This help"],
  ["Q", "Quit"],
];
const usedMemoryPercent = computed(() => memoryTotal.value
  ? Math.min(100, (memoryUsed.value ?? 0) / memoryTotal.value * 100)
  : 0);
const visibleCpuProcesses = computed(() => cpuProcesses.value.slice(0, 8));
const largestCpuProcess = computed(() => cpuProcesses.value[0]?.cpu_percent ?? 0);
const largestMemoryProcess = computed(() => memoryProcesses.value[0]?.memory_bytes ?? 0);
const hoveredShare = computed(() => {
  const tile = hoveredTile.value?.tile;
  if (!tile?.parent_bytes) return 100;
  return Math.min(100, (tile.bytes ?? 0) / tile.parent_bytes * 100);
});
const isHoveredMarked = computed(() => {
  const path = hoveredTile.value?.tile.path;
  return Boolean(path && marks.value.some((mark) => mark.path === path));
});
const coveredMarkName = computed(() => {
  const path = hoveredTile.value?.tile.path;
  if (!path) return "";
  const separator = path.includes("\\") ? "\\" : "/";
  const mark = marks.value.find((item) => path.startsWith(`${item.path}${separator}`));
  return mark?.path.split(separator).at(-1) ?? "";
});
const isHoveredCovered = computed(() => coveredMarkName.value.length > 0);

function receive(event: CliEvent): void {
  if (event.event === "ready") {
    status.value = "Live telemetry";
    void cli?.send({ id: crypto.randomUUID(), command: "get_settings" });
  }
  if (event.event === "settings") {
    if (typeof event.power === "string" && powerPresets.some((preset) => preset.id === event.power)) {
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
    void invoke("update_tray_status", {
      title: formatTrayTitle(cpu.value, usedMemoryPercent.value),
      summary: formatTraySummary(cpu.value, usedMemoryPercent.value, battery.value.present, battery.value.percent),
    });
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
    requestTreeView(treeRoot.value);
    startPendingVolumeNavigation();
  }
  if (event.event === "tree_cache_miss") {
    const path = String(event.path ?? event.root ?? "");
    void scan(path);
  }
  if (event.event === "tree_view") {
    treeRoot.value = String(event.root ?? treeRoot.value);
    files.value = Number(event.files ?? 0);
    dirs.value = Number(event.dirs ?? 0);
    bytes.value = Number(event.bytes ?? 0);
    tiles.value = Array.isArray(event.tiles) ? event.tiles as TreeTile[] : [];
    breadcrumbs.value = Array.isArray(event.breadcrumbs) ? event.breadcrumbs as Crumb[] : [];
    siblings.value = Array.isArray(event.siblings) ? event.siblings as Sibling[] : [];
    findings.value = Array.isArray(event.worth_a_look) ? event.worth_a_look as Finding[] : [];
    marks.value = Array.isArray(event.marks) ? event.marks as DiskMark[] : marks.value;
    selection.value = typeof event.selection === "object" && event.selection !== null ? event.selection as Record<string, unknown> : null;
    if (typeof event.unreadable === "number") unreadable.value = event.unreadable;
    const viewDir = breadcrumbs.value.at(-1)?.path ?? "";
    if (viewDir && viewDir !== lastViewDir) {
      lastViewDir = viewDir;
      zoom.value = { scale: 1, x: 0, y: 0 };
      if (!replayingHistory) {
        history.value = [...history.value.slice(0, historyIndex.value + 1), { root: treeRoot.value, path: viewDir }];
        historyIndex.value = history.value.length - 1;
      }
    }
    replayingHistory = false;
  }
  if (event.event === "marks_export") {
    const text = String(event.text ?? "");
    void navigator.clipboard?.writeText(text).catch(() => undefined);
    exportStatus.value = event.saved_path ? `SAVED ${String(event.saved_path)}` : `COPIED ${event.format === "prompt" ? "AGENT PROMPT" : "PATH LIST"}`;
  }
  if (event.event === "directory_chosen" && typeof event.path === "string") void scan(event.path);
  if (event.event === "reveal_done") status.value = "Live telemetry";
  if (event.event === "mark_changed" || event.event === "marks_changed") {
    marks.value = Array.isArray(event.marks) ? event.marks as DiskMark[] : [];
    review.value = null;
    if (event.event === "mark_changed") requestTreeView(treeRoot.value, String(event.path ?? treeRoot.value), treeRoot.value, false);
  }
  if (event.event === "review") {
    review.value = event;
    reviewScreen.value = true;
  }
  if (event.event === "removal_started") {
    removalStatus.value = "REMOVAL IN PROGRESS";
    removal.value = {
      phase: "running", total: Number(review.value?.targets ?? 0), completed: 0, removed: 0, bytes: 0, failed: 0,
      errors: [], freeBefore: selected.value?.available_bytes ?? null, freeAfter: null,
    };
  }
  if (event.event === "removal_progress" && removal.value) {
    removal.value.total = Number(event.total ?? removal.value.total);
    removal.value.completed = Number(event.completed ?? removal.value.completed);
  }
  if (event.event === "removal_item" && removal.value) {
    removal.value.completed += 1;
    if (event.ok === false) {
      removalStatus.value = `FAILED: ${String(event.error ?? "removal error")}`;
      removal.value.errors.push(`${String(event.path)} · ${String(event.error ?? "removal error")}`);
    }
  }
  if (event.event === "removal_complete") {
    removalStatus.value = `REMOVED ${String(event.removed)} · ${formatBytes(Number(event.bytes ?? 0))} · ${String(event.failed)} FAILED`;
    if (removal.value) {
      Object.assign(removal.value, { phase: "done", removed: Number(event.removed ?? 0), bytes: Number(event.bytes ?? 0), failed: Number(event.failed ?? 0) });
    }
    marks.value = [];
    review.value = null;
    removalConfirm.value = false;
    refresh();
  }
  if (event.event === "error") {
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
  volumeMenuOpen.value = false;
  tab.value = "disk";
  if (path === selectedVolume.value) {
    if (treeRoot.value !== path || tiles.value.length === 0) {
      requestTreeView(path, path, path, true);
    }
    return;
  }
  selectedVolume.value = path;
  if (activeScanId) {
    pendingVolumePath = path;
    void cli?.send({ id: crypto.randomUUID(), command: "cancel_scan" });
    return;
  }
  requestTreeView(path, path, path, true);
}

function startPendingVolumeNavigation(): void {
  const path = pendingVolumePath;
  if (!path || activeScanId) return;
  pendingVolumePath = undefined;
  requestTreeView(path, path, path);
}

function openDiskTree(): void {
  if (reviewScreen.value) cancelReview();
  tab.value = "disk";
  if (!scanning.value && tiles.value.length === 0 && selectedVolume.value) {
    requestTreeView(treeRoot.value || selectedVolume.value);
  }
}

function requestTreeView(
  path: string,
  selectedPath = path,
  root = treeRoot.value || selectedVolume.value,
  open = true,
): void {
  void cli?.send({
    id: crypto.randomUUID(), command: "tree_view", root, path: selectedPath, open,
    metric: treeMode.value === "files" ? "files" : "size", depth: treeDepth.value,
    width: treemapSize.value.width, height: treemapSize.value.height,
  });
}

function selectTile(tile: TreeTile, event: MouseEvent): void {
  if (!tile.path) return;
  const selectedBefore = selection.value?.path === tile.path;
  const open = tile.is_dir && (selectedBefore || event.detail > 1);
  requestTreeView(treeRoot.value, tile.path, treeRoot.value, open);
}

function setTreeMode(mode: "size" | "files" | "age"): void {
  treeMode.value = mode;
  const directory = breadcrumbs.value.at(-1)?.path ?? treeRoot.value;
  if (directory) requestTreeView(directory, directory, treeRoot.value, true);
}

function setTreeDepth(step: number): void {
  treeDepth.value = Math.max(1, Math.min(6, treeDepth.value + step));
  const directory = breadcrumbs.value.at(-1)?.path ?? treeRoot.value;
  if (directory) requestTreeView(directory, directory, treeRoot.value, true);
}

function rescanWithFilter(): void {
  if (treeRoot.value || selectedVolume.value) {
    void scan(treeRoot.value || selectedVolume.value);
  }
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
  powerMenuOpen.value = false;
  void cli?.send({ id: crypto.randomUUID(), command: "set_settings", power: preset });
}

function goHistory(step: number): void {
  const target = history.value[historyIndex.value + step];
  if (!target) return;
  historyIndex.value += step;
  replayingHistory = true;
  requestTreeView(target.path, target.path, target.root, true);
}

function openSibling(sibling: Sibling): void {
  crumbMenu.value = null;
  requestTreeView(sibling.is_dir ? sibling.path : treeRoot.value, sibling.path, treeRoot.value, sibling.is_dir);
}

function widenToVolume(): void {
  if (selectedVolume.value) requestTreeView(selectedVolume.value, selectedVolume.value, selectedVolume.value, true);
}

function openFilter(): void {
  filterOpen.value = true;
  void nextTick(() => filterInput.value?.focus());
}

function focusTreemap(): void {
  void nextTick(() => treemapCanvas.value?.focus());
}

function closeFilter(clear: boolean): void {
  if (clear) filterText.value = "";
  filterOpen.value = false;
  focusTreemap();
}

// Settings live in the CLI-owned ~/.karen/config.toml; the UI only applies and forwards them.
function applyUiZoom(next: number): void {
  uiZoom.value = Math.max(0.6, Math.min(2, Math.round(next * 100) / 100));
  document.documentElement.style.setProperty("zoom", String(uiZoom.value));
}

function setUiZoom(next: number): void {
  applyUiZoom(next);
  void cli?.send({ id: crypto.randomUUID(), command: "set_settings", ui_zoom: uiZoom.value });
}

function treeZoom(factor: number | null): void {
  const current = zoom.value;
  if (factor === null) { zoom.value = { scale: 1, x: 0, y: 0 }; return; }
  const scale = Math.max(1, Math.min(8, current.scale * factor));
  const cx = treemapSize.value.width / 2;
  const cy = treemapSize.value.height / 2;
  zoom.value = clampZoom(scale, cx - (cx - current.x) * scale / current.scale, cy - (cy - current.y) * scale / current.scale);
}

function clampZoom(scale: number, x: number, y: number): { scale: number; x: number; y: number } {
  const { width, height } = treemapSize.value;
  return { scale, x: Math.min(0, Math.max(width * (1 - scale), x)), y: Math.min(0, Math.max(height * (1 - scale), y)) };
}

function treeCoords(event: MouseEvent): { x: number; y: number; px: number; py: number } | null {
  const canvas = treemapCanvas.value;
  if (!canvas) return null;
  const bounds = canvas.getBoundingClientRect();
  const px = (event.clientX - bounds.left) * treemapSize.value.width / bounds.width;
  const py = (event.clientY - bounds.top) * treemapSize.value.height / bounds.height;
  return { px, py, x: (px - zoom.value.x) / zoom.value.scale, y: (py - zoom.value.y) / zoom.value.scale };
}

function wheelTree(event: WheelEvent): void {
  const point = treeCoords(event);
  if (!point) return;
  const current = zoom.value;
  if (event.shiftKey) {
    zoom.value = clampZoom(current.scale, current.x - (event.deltaY || event.deltaX), current.y);
    return;
  }
  const scale = Math.max(1, Math.min(8, current.scale * Math.exp(-event.deltaY * 0.0015)));
  if (scale === current.scale && event.deltaY > 0 && breadcrumbs.value.length > 1) {
    goToParent();
    return;
  }
  zoom.value = clampZoom(scale, point.px - (point.px - current.x) * scale / current.scale, point.py - (point.py - current.y) * scale / current.scale);
  const tile = tileAt(tiles.value, point.x, point.y);
  if (scale >= 3 && tile?.is_dir && tile.path) requestTreeView(tile.path, tile.path, treeRoot.value, true);
}

function actionPath(): string | null {
  const pointed = hoveredTile.value?.tile.path;
  if (pointerLast && pointed) return pointed;
  return typeof selection.value?.path === "string" ? selection.value.path : null;
}

function handleGlobalKeys(event: KeyboardEvent): void {
  const command = event.metaKey || event.ctrlKey;
  if (command && ["=", "+"].includes(event.key)) { event.preventDefault(); setUiZoom(uiZoom.value + 0.1); }
  else if (command && event.key === "-") { event.preventDefault(); setUiZoom(uiZoom.value - 0.1); }
  else if (command && event.key === "0") { event.preventDefault(); setUiZoom(1); }
  else if (tab.value !== "disk") return;
  else if (command && event.key.toLowerCase() === "o") { event.preventDefault(); chooseDirectory(); }
  else if ((event.altKey && event.key === "ArrowLeft") || (event.metaKey && event.key === "[")) { event.preventDefault(); goHistory(-1); }
  else if ((event.altKey && event.key === "ArrowRight") || (event.metaKey && event.key === "]")) { event.preventDefault(); goHistory(1); }
  else if (event.key === "Escape" && helpOpen.value) helpOpen.value = false;
}

function handleHistoryButtons(event: MouseEvent): void {
  if (tab.value !== "disk" || (event.button !== 3 && event.button !== 4)) return;
  event.preventDefault();
  goHistory(event.button === 3 ? -1 : 1);
}

function cancelReview(): void {
  review.value = null;
  reviewScreen.value = false;
  removalConfirm.value = false;
  void cli?.send({ id: crypto.randomUUID(), command: "cancel_review" });
}

function confirmRemoval(): void {
  void cli?.send({ id: crypto.randomUUID(), command: "confirm_remove", mode: removalMode.value });
  removalConfirm.value = false;
}

function closeVolumeMenu(event: PointerEvent): void {
  const target = event.target;
  if (target instanceof Element && !target.closest(".volume-picker, .volume-options")) {
    volumeMenuOpen.value = false;
  }
  if (target instanceof Element && !target.closest(".tree-crumb-wrap")) crumbMenu.value = null;
  if (target instanceof Element && !target.closest(".power-picker")) powerMenuOpen.value = false;
}

function positionVolumeMenu(): void {
  const trigger = volumePickerTrigger.value;
  if (!trigger) return;
  const bounds = trigger.getBoundingClientRect();
  const gap = 6;
  const edge = 16;
  const width = Math.min(410, window.innerWidth - edge * 2);
  const desiredHeight = Math.min(volumes.value.length * 48 + 10, window.innerHeight - edge * 2);
  const below = window.innerHeight - bounds.bottom - gap - edge;
  const above = bounds.top - gap - edge;
  volumeMenuAbove.value = below < desiredHeight && above > below;
  const maxHeight = Math.max(48, Math.min(desiredHeight, volumeMenuAbove.value ? above : below));
  const left = Math.max(edge, Math.min(bounds.right - width, window.innerWidth - width - edge));
  const top = volumeMenuAbove.value
    ? Math.max(edge, bounds.top - gap - maxHeight)
    : Math.min(window.innerHeight - edge - maxHeight, bounds.bottom + gap);
  volumeMenuStyle.value = {
    top: `${top}px`,
    left: `${left}px`,
    width: `${width}px`,
    maxHeight: `${maxHeight}px`,
  };
}

async function toggleVolumeMenu(): Promise<void> {
  volumeMenuOpen.value = !volumeMenuOpen.value;
  if (volumeMenuOpen.value) {
    await nextTick();
    positionVolumeMenu();
  }
}

function processCpuWidth(value: number): string {
  return `${largestCpuProcess.value > 0 ? Math.min(value / largestCpuProcess.value * 100, 100) : 0}%`;
}

function processMemoryWidth(value: number): string {
  return `${largestMemoryProcess.value > 0 ? Math.min(value / largestMemoryProcess.value * 100, 100) : 0}%`;
}

function closeVolumeMenuOnEscape(event: KeyboardEvent): void {
  if (event.key !== "Escape") return;
  if (removalConfirm.value || review.value) {
    cancelReview();
    return;
  }
  volumeMenuOpen.value = false;
}

function startInspectorResize(event: PointerEvent): void {
  const startX = event.clientX;
  const startWidth = inspectorWidth.value;
  const move = (next: PointerEvent): void => {
    inspectorWidth.value = Math.max(220, Math.min(520, startWidth + startX - next.clientX));
  };
  const stop = (): void => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", stop);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", stop);
}

function goToParent(): void {
  if (breadcrumbs.value.length <= 1) return;
  const parent = breadcrumbs.value[breadcrumbs.value.length - 2];
  requestTreeView(parent.path, parent.path, treeRoot.value, true);
}

function stepSibling(step: number): void {
  const list = siblings.value;
  const at = list.findIndex((sibling) => sibling.path === selection.value?.path);
  const next = list[(at + step + list.length) % list.length];
  if (next) requestTreeView(treeRoot.value, next.path, treeRoot.value, false);
}

const vimArrows: Record<string, string> = { h: "ArrowLeft", j: "ArrowDown", k: "ArrowUp", l: "ArrowRight" };

function handleDiskKeyboard(event: KeyboardEvent): void {
  if (tab.value !== "disk" || reviewScreen.value || event.metaKey || event.ctrlKey || event.altKey) return;
  const target = event.target;
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return;
  if (target instanceof Element && target.closest("button:not(.tree-tile)")) return;

  if (helpOpen.value) return;
  const key = vimArrows[event.key] ?? event.key;
  // Moving the selection by keyboard hands the action target back from the pointer; other keys act on the pointed tile.
  if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Tab"].includes(key)) pointerLast = false;
  if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(key)) {
    const current = tiles.value.find((tile) => tile.path === selection.value?.path);
    const candidates = tiles.value.filter((tile) => tile.path && tile.path !== current?.path);
    const origin = current ?? candidates[0];
    if (!origin) return;
    const direction = key === "ArrowLeft" ? [-1, 0]
      : key === "ArrowRight" ? [1, 0]
        : key === "ArrowUp" ? [0, -1] : [0, 1];
    const center = (tile: TreeTile): [number, number] => [tile.x + tile.width / 2, tile.y + tile.height / 2];
    const [ox, oy] = center(origin);
    const next = candidates
      .map((tile) => {
        const [x, y] = center(tile);
        const dx = x - ox;
        const dy = y - oy;
        const primary = dx * direction[0] + dy * direction[1];
        const offset = Math.abs(dx * direction[1] - dy * direction[0]);
        return { tile, score: primary > 0 ? primary * 2.5 + offset : Number.POSITIVE_INFINITY };
      })
      .filter((item) => Number.isFinite(item.score))
      .sort((left, right) => left.score - right.score)[0]?.tile;
    if (next?.path) {
      event.preventDefault();
      requestTreeView(treeRoot.value, next.path, treeRoot.value, false);
    }
    return;
  }

  const actionTarget = actionPath();
  if (event.key === "Enter" && actionTarget) {
    const targetTile = tiles.value.find((tile) => tile.path === actionTarget);
    if (targetTile?.is_dir) {
      event.preventDefault();
      requestTreeView(treeRoot.value, actionTarget, treeRoot.value, true);
    }
  } else if ((event.code === "Space" || event.key === "x") && actionTarget) {
    event.preventDefault();
    toggleMark(actionTarget);
  } else if (event.key === "/" || event.key === "s") {
    event.preventDefault();
    openFilter();
  } else if (event.key === "?") {
    helpOpen.value = true;
  } else if (event.key === "g") {
    widenToVolume();
  } else if (event.key === "o") {
    revealPath(actionTarget);
  } else if (event.key === "c" && marks.value.length) {
    requestReview();
  } else if (event.key === "q") {
    closeWindow();
  } else if (event.key === "-") {
    treeZoom(1 / 1.25);
  } else if (event.key === "=" || event.key === "+") {
    treeZoom(1.25);
  } else if (event.key === "0") {
    treeZoom(null);
  } else if (event.key === "Backspace" || event.key === "u") {
    event.preventDefault();
    goToParent();
  } else if (event.key === "Tab") {
    event.preventDefault();
    stepSibling(event.shiftKey ? -1 : 1);
  } else if (event.key === "Escape") {
    if (filterText.value) closeFilter(true);
    else goToParent();
  } else if (event.key === "p") {
    inspectorOpen.value = !inspectorOpen.value;
  } else if (event.key === "d" && !scanning.value) {
    apparentSize.value = !apparentSize.value;
    rescanWithFilter();
  } else if (event.key === "i" && !scanning.value) {
    includeHidden.value = !includeHidden.value;
    rescanWithFilter();
  } else if (event.key === "[") {
    event.preventDefault();
    setTreeDepth(-1);
  } else if (event.key === "]") {
    event.preventDefault();
    setTreeDepth(1);
  } else if (event.key.toLowerCase() === "t") {
    const index = treeModes.indexOf(treeMode.value);
    setTreeMode(treeModes[(index + 1) % treeModes.length]);
  } else if (event.key.toLowerCase() === "r" && !scanning.value) {
    void scan(treeRoot.value || selectedVolume.value);
  }
}

function syncViewportWidth(): void {
  viewportWidth.value = window.innerWidth;
}

function startDragging(): void {
  void getCurrentWindow().startDragging();
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

function meterStyle(value: number): Record<string, string> {
  return { "--reading": `${Math.max(0, Math.min(value, 100))}%` };
}

function showTileTooltip(tile: TreeTile, event: PointerEvent): void {
  const width = 260;
  const height = 142;
  const gap = 12;
  hoveredTile.value = {
    tile,
    x: event.clientX + gap + width > window.innerWidth
      ? Math.max(8, event.clientX - gap - width)
      : event.clientX + gap,
    y: event.clientY + gap + height > window.innerHeight
      ? Math.max(8, event.clientY - gap - height)
      : event.clientY + gap,
  };
}

function updateTreePointer(event: PointerEvent): void {
  const canvas = treemapCanvas.value;
  if (!canvas) return;
  pointerLast = true;
  const point = treeCoords(event);
  const tile = point ? tileAt(tiles.value, point.x, point.y) : undefined;
  if (!tile) {
    hoveredTile.value = null;
    return;
  }
  showTileTooltip(tile, event);
}

function clickTreeCanvas(event: MouseEvent): void {
  const canvas = treemapCanvas.value;
  if (!canvas) return;
  const point = treeCoords(event);
  const tile = point ? tileAt(tiles.value, point.x, point.y) : undefined;
  if (!tile?.path) return;
  if (event.ctrlKey || event.metaKey || event.button === 1) {
    toggleMark(tile.path);
    return;
  }
  selectTile(tile, event);
}

function contextTree(event: MouseEvent): void {
  const point = treeCoords(event);
  const tile = point ? tileAt(tiles.value, point.x, point.y) : undefined;
  if (!tile?.path) return;
  requestTreeView(treeRoot.value, tile.path, treeRoot.value, false);
  revealPath(tile.path);
}

function drawCurrentTreemap(): void {
  if (!treemapCanvas.value) return;
  drawTreemap({
    canvas: treemapCanvas.value,
    tiles: tiles.value,
    marks: marks.value,
    selectedPath: typeof selection.value?.path === "string" ? selection.value.path : null,
    hoveredPath: hoveredTile.value?.tile.path ?? null,
    mode: treeMode.value,
    width: treemapSize.value.width,
    height: treemapSize.value.height,
    filter: filterText.value,
    zoom: zoom.value,
  });
}

async function observeTreemap(): Promise<void> {
  await nextTick();
  const canvas = treemapCanvas.value;
  if (!canvas) return;
  treemapResizeObserver?.disconnect();
  treemapResizeObserver = new ResizeObserver(([entry]) => {
    const width = Math.max(320, Math.round(entry.contentRect.width));
    const height = Math.max(240, Math.round(entry.contentRect.height));
    if (width === treemapSize.value.width && height === treemapSize.value.height) return;
    treemapSize.value = { width, height };
    const directory = breadcrumbs.value.at(-1)?.path ?? treeRoot.value;
    if (directory) requestTreeView(directory, directory, treeRoot.value, true);
  });
  treemapResizeObserver.observe(canvas);
}

watch([tiles, marks, selection, hoveredTile, treeMode, treemapSize, filterText, zoom], drawCurrentTreemap, { deep: true });
// Leaving Review unmounts the treemap canvas; the replacement element needs its own observer and a repaint.
watch(reviewScreen, async (open) => {
  if (open || tab.value !== "disk") return;
  await observeTreemap();
  drawCurrentTreemap();
});
watch(tab, (value) => {
  if (value === "disk") void observeTreemap();
  else {
    hoveredTile.value = null;
    treemapResizeObserver?.disconnect();
  }
});

const tabItems = computed<TabItem[]>(() => [
  { id: "runtime", index: "01", label: "Runtime", badge: cpu.value === null ? "WAIT" : "LIVE" },
  { id: "cpu", index: "02", label: "CPU", badge: cpuProcessesAvailable.value ? "LIVE" : "WAIT" },
  { id: "memory", index: "03", label: "Memory", badge: memoryTotal.value === null ? "WAIT" : "LIVE" },
  { id: "battery", index: "04", label: "Battery", badge: battery.value.present ? `${battery.value.percent ?? "—"}%` : "AC" },
  { id: "network", index: "05", label: "Network", badge: network.value.rx_bytes_per_sec == null ? "WAIT" : "LIVE" },
  { id: "disk", index: "06", label: "Space", badge: scanning.value ? "MAP" : treeRoot.value ? "READY" : "OPEN" },
]);

function selectTab(target: TabId): void {
  if (target === "disk") openDiskTree();
  else tab.value = target;
}

function pushSample(samples: number[], value: number): void {
  samples.push(value);
  if (samples.length > 90) samples.shift();
}

function chartPath(samples: number[], maximum: number): string {
  if (samples.length < 2) return "";
  return samples.map((value, index) => {
    const x = index / (samples.length - 1) * 1000;
    const y = 100 - Math.min(value / maximum, 1) * 100;
    return `${index === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
}

function formatBytes(value: number): string {
  if (value < 1024) return `${value} B`;
  const units = ["KB", "MB", "GB", "TB", "PB"];
  let amount = value;
  let index = -1;
  do { amount /= 1024; index += 1; } while (amount >= 1024 && index < units.length - 1);
  return `${amount.toFixed(1)} ${units[index]}`;
}

function formatRate(value: number | null | undefined): string {
  return typeof value === "number" ? `${formatBytes(value)}/s` : "—";
}

function formatDuration(secs: number | null | undefined): string {
  if (typeof secs !== "number" || secs <= 0) return "—";
  return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
}

const networkPeakRate = computed(() => Math.max(1024, ...networkRxSamples.value, ...networkTxSamples.value));

function selectTrayTab(target: string): void {
  if (target === "space") selectTab("disk");
  else if (["runtime", "cpu", "memory", "battery", "network"].includes(target)) {
    selectTab(target as TabId);
  }
}

function volumePercent(volume: Volume): number {
  const total = volume.total_bytes ?? 0;
  const free = volume.free_bytes ?? 0;
  return total > 0 ? Math.min(100, Math.max(0, ((total - free) / total) * 100)) : 0;
}

onMounted(async () => {
  window.addEventListener("pointerdown", closeVolumeMenu);
  window.addEventListener("resize", positionVolumeMenu);
  window.addEventListener("resize", syncViewportWidth);
  window.addEventListener("scroll", positionVolumeMenu, true);
  window.addEventListener("keydown", closeVolumeMenuOnEscape);
  window.addEventListener("keydown", handleDiskKeyboard);
  window.addEventListener("keydown", handleGlobalKeys);
  window.addEventListener("mouseup", handleHistoryButtons);
  if (!isTauri()) {
    status.value = "Open Karen.app to connect the CLI";
    return;
  }
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
  treemapResizeObserver?.disconnect();
  window.removeEventListener("pointerdown", closeVolumeMenu);
  window.removeEventListener("resize", positionVolumeMenu);
  window.removeEventListener("scroll", positionVolumeMenu, true);
  window.removeEventListener("keydown", closeVolumeMenuOnEscape);
  window.removeEventListener("keydown", handleDiskKeyboard);
  window.removeEventListener("keydown", handleGlobalKeys);
  window.removeEventListener("mouseup", handleHistoryButtons);
  window.removeEventListener("resize", syncViewportWidth);
  if (refreshTimer !== undefined) window.clearInterval(refreshTimer);
  void cli?.stop();
});
</script>

<template>
  <main class="instrument" :class="{ macos: isMacOS }">
    <header class="masthead" @mousedown.self="startDragging">
      <div class="brand-lockup" @mousedown.self="startDragging">
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
        <span class="brand-name">KAREN</span>
        <span class="brand-edition">SYSTEM MONITOR</span>
      </div>
      <div class="masthead-right" @mousedown.self="startDragging">
        <div class="telemetry-state" :class="{ fault: status.includes('fault') || status.includes('unavailable') }">
          <span class="state-lamp"></span><span>{{ status }}</span><span class="state-divider">/</span><span>LOCAL</span>
        </div>
        <div class="window-controls" aria-label="Window controls">
          <button aria-label="Minimize window" title="Minimize" @mousedown.stop @click.stop="minimizeWindow"><i class="minimize-glyph"></i></button>
          <button aria-label="Toggle window size" title="Toggle size" @mousedown.stop @click.stop="toggleMaximizeWindow"><i class="maximize-glyph"></i></button>
          <button class="close-control" aria-label="Close window" title="Close" @mousedown.stop @click.stop="closeWindow"><i class="close-glyph"></i></button>
        </div>
      </div>
    </header>

    <TabWheel :tabs="tabItems" :active="tab" @select="selectTab" />

    <section v-if="tab === 'runtime'" class="view metric-view runtime-view">
      <div class="view-heading"><h1>Runtime Monitor</h1></div>
      <div class="metric-console runtime-console">
        <div class="metric-main"><span class="metric-caption">TOTAL CPU LOAD</span><strong class="metric-value">{{ cpu ?? "—" }}<small>%</small></strong><div class="metric-ruler"><i :style="{ width: `${cpu ?? 0}%` }"></i></div><div class="metric-state"><span>LOAD CONDITION</span><b>{{ cpu === null ? "INITIALIZING" : cpu > 85 ? "HIGH LOAD" : cpu > 50 ? "ACTIVE" : "NOMINAL" }}</b></div></div>
        <div class="chart-console"><div class="chart-heading"><span>PROCESSOR LOAD</span><span>3 SECOND SAMPLE <i>/</i> 4.5 MIN WINDOW</span></div><svg class="history-chart" viewBox="0 0 1000 100" preserveAspectRatio="none" role="img" aria-label="Recent processor load history"><path class="chart-grid" d="M0 1H1000 M0 50H1000 M0 99H1000"/><path v-if="cpuSamples.length > 1" class="chart-area" :d="`${chartPath(cpuSamples, 100)} L1000,100 L0,100 Z`"/><path class="chart-line cpu-line" :d="chartPath(cpuSamples, 100)"/></svg><div class="chart-axis"><span>−4.5 MIN</span><span>−2 MIN</span><span>NOW</span></div><div class="process-ledger"><div class="process-ledger-heading"><span>CPU BY PROCESS</span><span>100% = ONE CORE</span></div><p v-if="!cpuProcessesAvailable" class="process-empty">SAMPLING BASELINE — PROCESS LOAD WARMING UP</p><p v-else-if="!visibleCpuProcesses.length" class="process-empty">NO PROCESS CPU READINGS AVAILABLE</p><div v-else class="process-rows"><div v-for="process in visibleCpuProcesses" :key="process.pid" class="process-row"><span class="process-name" :title="process.name">{{ process.name }}</span><span class="process-pid">PID {{ process.pid }}</span><b>{{ process.cpu_percent.toFixed(1) }}%</b></div></div></div></div>
        <div class="runtime-memory"><span class="metric-caption">MEMORY IN USE</span><strong>{{ memoryUsed === null ? "—" : formatBytes(memoryUsed) }}</strong><div class="metric-ruler memory-ruler"><i :style="{ width: `${usedMemoryPercent}%` }"></i></div><div class="runtime-memory-stats"><span>USED <b>{{ usedMemoryPercent.toFixed(0) }}%</b></span><span>TOTAL <b>{{ memoryTotal === null ? "—" : formatBytes(memoryTotal) }}</b></span><span>AVAILABLE <b>{{ memoryTotal === null ? "—" : formatBytes(memoryTotal - (memoryUsed ?? 0)) }}</b></span></div><div class="runtime-process-mini"><span class="metric-caption">RESIDENT MEMORY LEADERS</span><div v-for="process in memoryProcesses.slice(0, 3)" :key="process.pid"><span>{{ process.name }}</span><b>{{ formatBytes(process.memory_bytes) }}</b></div><p v-if="!memoryProcessesAvailable">PROCESS SAMPLE UNAVAILABLE</p></div><button class="runtime-memory-link" @click="tab = 'memory'">MEMORY DETAIL <span>↗</span></button></div>
        <div class="runtime-aux">
          <button class="runtime-mini-instrument runtime-battery-card" @click="tab = 'battery'">
            <span class="metric-caption">POWER &amp; BATTERY</span>
            <strong>{{ battery.present ? `${battery.percent ?? "—"}%` : "AC POWER" }}</strong>
            <small>{{ battery.present ? `${(battery.state ?? "unknown").toUpperCase()} · ${battery.power_watts != null ? `${battery.power_watts.toFixed(1)} W` : "— W"}` : "No battery detected (AC power)" }}</small>
            <span class="runtime-mini-open">BATTERY DETAIL ↗</span>
          </button>
          <button class="runtime-mini-instrument runtime-network-card" @click="tab = 'network'">
            <span class="metric-caption">NETWORK THROUGHPUT</span>
            <strong>RX {{ formatRate(network.rx_bytes_per_sec) }}</strong>
            <small>TX {{ formatRate(network.tx_bytes_per_sec) }} · {{ network.interfaces.length }} IFACES</small>
            <span class="runtime-mini-open">NETWORK DETAIL ↗</span>
          </button>
        </div>
      </div>
      <section class="runtime-volumes"><div class="instrument-section-heading"><span>VOLUME CAPACITY INSTRUMENTS</span><span>{{ volumes.length.toString().padStart(2, "0") }} MOUNTED</span></div><div class="runtime-volume-grid"><button v-for="(volume, index) in volumes" :key="volume.mount_point" class="runtime-volume" @click="selectVolume(volume.mount_point)"><span class="runtime-volume-index">{{ String(index + 1).padStart(2, "0") }}</span><span class="runtime-volume-name">{{ volume.mount_point }}</span><span class="runtime-volume-device">{{ volume.device || "LOCAL STORAGE" }}</span><strong>{{ volumePercent(volume).toFixed(0) }}<small>% USED</small></strong><span class="runtime-volume-track" :style="meterStyle(volumePercent(volume))"><i></i></span><span class="runtime-volume-capacity">{{ formatBytes((volume.total_bytes ?? 0) - (volume.free_bytes ?? 0)) }} USED <b>{{ volume.available_bytes == null ? "—" : formatBytes(volume.available_bytes) }} FREE</b></span><span class="runtime-volume-open">OPEN SPACE ↗</span></button><p v-if="!volumes.length" class="process-empty">WAITING FOR MOUNTED VOLUME TELEMETRY</p></div></section>
      <div class="panel-status"><span class="calibration">✥</span> LIVE SYSTEM TELEMETRY <i>·</i> CPU · MEMORY · BATTERY · NETWORK · {{ volumes.length }} VOLUMES <i>·</i> UPDATED EVERY 3 SECONDS</div>
    </section>

    <section v-else-if="tab === 'cpu'" class="view metric-view detail-view">
      <div class="view-heading"><h1>CPU Process Load</h1><span class="volume-count">SYSTEM {{ cpu ?? "—" }}% · 100% = ONE CORE</span></div>
      <div class="detail-process-console"><div class="detail-table-heading"><span>RANK / PROCESS</span><span>PID</span><span>CPU SHARE</span></div><div class="detail-process-rows"><p v-if="!cpuProcessesAvailable" class="process-empty">SAMPLING BASELINE — PROCESS LOAD WARMING UP</p><p v-else-if="!cpuProcesses.length" class="process-empty">NO PROCESS CPU READINGS AVAILABLE</p><div v-for="(process, index) in cpuProcesses" :key="process.pid" class="detail-process-row"><span class="detail-process-identity"><i>{{ String(index + 1).padStart(2, "0") }}</i><b :title="process.name">{{ process.name }}</b></span><span class="process-pid">{{ process.pid }}</span><span class="detail-process-reading"><b>{{ process.cpu_percent.toFixed(1) }}%</b><i :style="{ width: processCpuWidth(process.cpu_percent) }"></i></span></div></div></div>
      <div class="panel-status"><span class="calibration">✥</span> PROCESS CPU <i>·</i> DESCENDING LOAD <i>·</i> TOP 20 <i>·</i> 3 SECOND SAMPLE</div>
    </section>

    <section v-else-if="tab === 'memory'" class="view metric-view memory-view">
      <div class="view-heading"><h1>Memory</h1></div>
      <div class="metric-console memory-console">
        <div class="metric-main"><span class="metric-caption">CAPACITY IN USE</span><strong class="metric-value memory-value">{{ memoryUsed === null ? "—" : formatBytes(memoryUsed) }}</strong><div class="metric-ruler memory-ruler"><i :style="{ width: `${usedMemoryPercent}%` }"></i></div><div class="metric-state"><span>INSTALLED CAPACITY</span><b>{{ memoryTotal === null ? "INITIALIZING" : formatBytes(memoryTotal) }}</b></div><div class="metric-state"><span>AVAILABLE RESERVE</span><b>{{ memoryTotal === null ? "—" : formatBytes(memoryTotal - (memoryUsed ?? 0)) }}</b></div></div>
        <div class="chart-console"><div class="chart-heading"><span>MEMORY PRESSURE</span><span>3 SECOND SAMPLE <i>/</i> 4.5 MIN WINDOW</span></div><svg class="history-chart" viewBox="0 0 1000 100" preserveAspectRatio="none" role="img" aria-label="Recent memory use history"><path class="chart-grid" d="M0 1H1000 M0 50H1000 M0 99H1000"/><path v-if="memorySamples.length > 1" class="chart-area memory-area" :d="`${chartPath(memorySamples, 100)} L1000,100 L0,100 Z`"/><path class="chart-line memory-line" :d="chartPath(memorySamples, 100)"/></svg><div class="chart-axis"><span>−4.5 MIN</span><span>−2 MIN</span><span>NOW</span></div></div>
      </div>
      <div class="detail-process-console memory-process-console"><div class="instrument-section-heading"><span>PROCESS RESIDENT MEMORY</span><span>RSS / WORKING SET · TOP 20</span></div><div class="detail-table-heading"><span>RANK / PROCESS</span><span>PID</span><span>RESIDENT</span></div><div class="detail-process-rows"><p v-if="!memoryProcessesAvailable" class="process-empty">PROCESS MEMORY SAMPLE UNAVAILABLE</p><p v-else-if="!memoryProcesses.length" class="process-empty">NO PROCESS MEMORY READINGS AVAILABLE</p><div v-for="(process, index) in memoryProcesses" :key="process.pid" class="detail-process-row"><span class="detail-process-identity"><i>{{ String(index + 1).padStart(2, "0") }}</i><b :title="process.name">{{ process.name }}</b></span><span class="process-pid">{{ process.pid }}</span><span class="detail-process-reading"><b>{{ formatBytes(process.memory_bytes) }}</b><i :style="{ width: processMemoryWidth(process.memory_bytes) }"></i></span></div></div></div>
      <div class="panel-status"><span class="calibration">✥</span> PHYSICAL MEMORY <i>·</i> {{ usedMemoryPercent.toFixed(0) }}% COMMITTED <i>·</i> LOCAL SAMPLING</div>
    </section>

    <section v-else-if="tab === 'battery'" class="view metric-view battery-view">
      <div class="view-heading"><h1>Battery &amp; Power</h1><span class="volume-count">{{ battery.present ? `${(battery.state ?? "unknown").toUpperCase()} · ${battery.percent ?? "—"}%` : "NO BATTERY DETECTED (AC POWER)" }}</span></div>
      <div class="metric-console memory-console battery-console">
        <div class="metric-main">
          <span class="metric-caption">CHARGE RESERVE</span>
          <strong class="metric-value memory-value">{{ battery.present ? `${battery.percent ?? "—"}%` : "AC" }}</strong>
          <div class="metric-ruler"><i :style="{ width: `${battery.present ? (battery.percent ?? 0) : 100}%` }"></i></div>
          <div class="metric-state"><span>SUPPLY STATE</span><b>{{ battery.present ? (battery.state ?? "unknown").toUpperCase() : "NO BATTERY DETECTED (AC POWER)" }}</b></div>
          <div class="metric-state"><span>POWER RATE</span><b>{{ battery.power_watts != null ? `${battery.power_watts.toFixed(1)} W` : "—" }}</b></div>
          <div class="metric-state"><span>ESTIMATED RUNTIME</span><b>{{ formatDuration(battery.time_to_empty_secs ?? battery.time_to_full_secs) }}</b></div>
        </div>
        <div class="chart-console">
          <div class="chart-heading"><span>CHARGE &amp; WATTAGE TRACE</span><span>3 SECOND SAMPLE <i>/</i> 4.5 MIN WINDOW</span></div>
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
        <div class="instrument-section-heading"><span>CELL CAPACITY &amp; ELECTRICAL TELEMETRY</span><span>{{ battery.present ? "SMART BATTERY" : "DESKTOP / AC LINE" }}</span></div>
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
      <div class="panel-status"><span class="calibration">✥</span> POWER TELEMETRY <i>·</i> {{ battery.present ? `HEALTH ${battery.health_percent ?? "—"}%` : "AC MAINS" }} <i>·</i> LOCAL SAMPLING</div>
    </section>

    <section v-else-if="tab === 'network'" class="view metric-view network-view">
      <div class="view-heading"><h1>Network Throughput</h1><span class="volume-count">RX {{ formatRate(network.rx_bytes_per_sec) }} · TX {{ formatRate(network.tx_bytes_per_sec) }}</span></div>
      <div class="metric-console memory-console network-console">
        <div class="metric-main">
          <span class="metric-caption">EXTERNAL THROUGHPUT</span>
          <strong class="metric-value memory-value">{{ formatRate(network.rx_bytes_per_sec) }}</strong>
          <div class="metric-state"><span>TRANSMIT RATE (TX)</span><b>{{ formatRate(network.tx_bytes_per_sec) }}</b></div>
          <div class="metric-state"><span>CUMULATIVE RECEIVED</span><b>{{ formatBytes(network.total_rx_bytes) }}</b></div>
          <div class="metric-state"><span>CUMULATIVE TRANSMITTED</span><b>{{ formatBytes(network.total_tx_bytes) }}</b></div>
        </div>
        <div class="chart-console">
          <div class="chart-heading"><span>DUAL-CHANNEL RX / TX TRACE</span><span>PEAK SCALE {{ formatRate(networkPeakRate) }}</span></div>
          <svg class="history-chart" viewBox="0 0 1000 100" preserveAspectRatio="none" role="img" aria-label="Recent network receive and transmit history">
            <path class="chart-grid" d="M0 1H1000 M0 50H1000 M0 99H1000"/>
            <path v-if="networkRxSamples.length > 1" class="chart-area memory-area" :d="`${chartPath(networkRxSamples, networkPeakRate)} L1000,100 L0,100 Z`"/>
            <path class="chart-line memory-line" :d="chartPath(networkRxSamples, networkPeakRate)"/>
            <path class="chart-line cpu-line" :d="chartPath(networkTxSamples, networkPeakRate)"/>
          </svg>
          <div class="chart-axis"><span>−4.5 MIN</span><span>−2 MIN</span><span>NOW</span></div>
        </div>
      </div>
      <div class="detail-process-console memory-process-console network-interface-console">
        <div class="instrument-section-heading"><span>NETWORK INTERFACES</span><span>SORTED BY LIVE THROUGHPUT · TOP 20</span></div>
        <div class="detail-table-heading network-table-heading"><span>RANK / INTERFACE</span><span>RX RATE</span><span>TX RATE</span><span>TOTAL RX / TX</span><span>PACKETS / ERR</span></div>
        <div class="detail-process-rows">
          <p v-if="!network.interfaces.length" class="process-empty">NO NETWORK INTERFACES DETECTED</p>
          <div v-for="(iface, index) in network.interfaces" :key="iface.name" class="detail-process-row network-interface-row">
            <span class="detail-process-identity"><i>{{ String(index + 1).padStart(2, "0") }}</i><b :title="iface.name">{{ iface.name }}</b></span>
            <span class="process-pid">{{ formatRate(iface.rx_bytes_per_sec) }}</span>
            <span class="process-pid">{{ formatRate(iface.tx_bytes_per_sec) }}</span>
            <span class="process-pid">{{ formatBytes(iface.total_rx_bytes) }} / {{ formatBytes(iface.total_tx_bytes) }}</span>
            <span class="process-pid">{{ iface.packets_rx + iface.packets_tx }} pkts · {{ iface.errors_rx + iface.errors_tx }} err</span>
          </div>
        </div>
      </div>
      <div class="panel-status"><span class="calibration">✥</span> NETWORK TELEMETRY <i>·</i> {{ network.interfaces.length }} INTERFACES <i>·</i> 3 SECOND SAMPLE</div>
    </section>

    <section v-else class="view disk-view" :class="{ 'no-inspector': !showInspector }" :style="{ '--inspector-w': `${inspectorWidth}px` }">
      <div v-if="!reviewScreen" class="tree-console">
        <div class="tree-toolbar"><div class="tree-location"><div class="volume-picker"><button ref="volumePickerTrigger" class="volume-picker-trigger" aria-haspopup="listbox" :aria-expanded="volumeMenuOpen" @click="toggleVolumeMenu"><i class="drive-lamp"></i><strong>{{ selectedVolume || "NO VOLUME" }}</strong><small>{{ selected?.device || "LOCAL STORAGE" }}</small><i class="picker-chevron" :class="{ open: volumeMenuOpen }"></i></button></div><span class="location-lamp"></span><div class="history-buttons"><button aria-label="History back" :title="history[historyIndex - 1]?.path ?? 'Nothing earlier'" :disabled="historyIndex <= 0" @click="goHistory(-1)">‹</button><button aria-label="History forward" :title="history[historyIndex + 1]?.path ?? 'Nothing later'" :disabled="historyIndex >= history.length - 1" @click="goHistory(1)">›</button></div><template v-for="item in visibleCrumbs" :key="item.gap ? 'gap' : item.crumb.path"><span v-if="item.gap" class="tree-crumb-gap" aria-hidden="true">…</span><span v-else class="tree-crumb-wrap"><button class="tree-crumb" :title="item.crumb.path" @click="requestTreeView(item.crumb.path, item.crumb.path, treeRoot, true)">{{ item.crumb.label }}</button><button v-if="(item.crumb.siblings?.length ?? 0) > 1" class="crumb-siblings" :aria-label="`Siblings of ${item.crumb.label}`" :aria-expanded="crumbMenu === item.index" @click.stop="crumbMenu = crumbMenu === item.index ? null : item.index">▾</button><div v-if="crumbMenu === item.index" class="crumb-menu" role="listbox" :aria-label="`Siblings of ${item.crumb.label}`"><button v-for="sibling in item.crumb.siblings" :key="sibling.path" role="option" :class="{ chosen: sibling.path === item.crumb.path }" @click="openSibling(sibling)"><i class="crumb-dot" :style="{ background: categoryColor(sibling.category ?? '') }"></i><span class="crumb-name">{{ sibling.name }}</span><i class="crumb-bar"><b :style="{ width: `${Math.max(2, sibling.bytes / (item.crumb.siblings?.[0]?.bytes || 1) * 100)}%` }"></b></i><span class="crumb-size">{{ formatBytes(sibling.bytes) }}</span></button></div></span></template></div><div class="tree-actions"><div class="power-picker"><button class="power-trigger" aria-haspopup="listbox" :aria-expanded="powerMenuOpen" title="Power Efficiency: scan worker preset" @click="powerMenuOpen = !powerMenuOpen">POWER · {{ powerLabel.toUpperCase() }}{{ workerLabel }}</button><div v-if="powerMenuOpen" class="power-menu" role="listbox" aria-label="Power Efficiency"><button v-for="preset in powerPresets" :key="preset.id" role="option" :class="{ chosen: power === preset.id }" @click="setPower(preset.id)"><b>{{ preset.label }}</b><small>{{ preset.note }}</small></button></div></div><span v-if="previewing" class="tree-preview-badge">SCANNING · PREVIOUS MAP RETAINED</span><button class="map-button" :disabled="scanning || !selectedVolume" @click="scan(treeRoot || selectedVolume)">{{ scanning ? "MAPPING…" : treeRoot ? "RESCAN" : "MAP VOLUME" }}</button></div></div>
        <div class="tree-controls" @click="($event.target as HTMLElement).closest('button, label') && focusTreemap()"><div class="tree-mode" role="group" aria-label="Treemap mode"><button v-for="mode in treeModes" :key="mode" :class="{ active: treeMode === mode }" @click="setTreeMode(mode)">{{ mode.toUpperCase() }}</button></div><label class="tree-check"><input v-model="includeHidden" type="checkbox" :disabled="scanning" @change="rescanWithFilter"><span>HIDDEN FILES</span></label><label class="tree-check"><input v-model="apparentSize" type="checkbox" :disabled="scanning" @change="rescanWithFilter"><span>APPARENT SIZE</span></label><label v-show="filterOpen || filterText" class="tree-filter"><span>FILTER</span><input ref="filterInput" v-model="filterText" type="text" placeholder="name contains…" aria-label="Filter tiles by name" @keydown.enter.prevent="closeFilter(false)" @keydown.esc.stop.prevent="closeFilter(true)"></label><div class="depth-control"><span>DEPTH {{ treeDepth }}</span><button aria-label="Decrease treemap depth" :disabled="treeDepth <= 1" @click="setTreeDepth(-1)">−</button><button aria-label="Increase treemap depth" :disabled="treeDepth >= 6" @click="setTreeDepth(1)">+</button></div></div>
        <div class="tree-legend" :class="{ 'age-mode': treeMode === 'age' }"><span v-if="treeRoot" class="tree-count">{{ previewing ? `SCAN ${scanProgress.files.toLocaleString()} FILES` : formatBytes(bytes) }} <i>·</i> {{ files.toLocaleString() }} files <i>·</i> {{ dirs.toLocaleString() }} dirs<template v-if="unreadable"> <i>·</i> {{ unreadable.toLocaleString() }} unreadable</template><template v-if="filterText"> <i>·</i> filter “{{ filterText }}”</template></span><span class="age-legend"><i></i> THIS WEEK <i></i> THIS MONTH <i></i> SIX MONTHS <i></i> THIS YEAR <i></i> OLDER</span><span><i class="legend-code"></i> CODE</span><span><i class="legend-agent"></i> AGENT SCRATCH</span><span><i class="legend-tool"></i> TOOLCHAINS</span><span><i class="legend-synced"></i> SYNCED</span><span><i class="legend-git"></i> GIT</span><span><i class="legend-media"></i> MEDIA</span><span><i class="legend-documents"></i> DOCUMENTS</span><span><i class="legend-cache"></i> CACHE</span><span><i class="legend-other"></i> OTHER</span></div>
        <div class="treemap-frame" :class="{ mapped: tiles.length > 0 }">
          <canvas v-show="tiles.length" ref="treemapCanvas" class="treemap-canvas" tabindex="0" role="img" aria-label="Disk usage treemap. Use arrow keys to select a tile, Enter to open, and Space to mark." @pointermove="updateTreePointer" @pointerleave="hoveredTile = null" @click="clickTreeCanvas" @auxclick.prevent="clickTreeCanvas" @contextmenu.prevent="contextTree" @wheel.prevent="wheelTree"></canvas>
          <div v-if="!tiles.length" class="tree-empty"><span class="empty-state-label">{{ scanning ? "MAPPING STORAGE" : treeRoot ? "NO DIRECTORY ENTRIES" : "VOLUME NOT MAPPED" }}</span><p>{{ scanning ? `${scanProgress.files.toLocaleString()} files · ${formatBytes(scanProgress.bytes)} inspected${previewing ? " · previous completed map retained" : ""}` : treeRoot ? "Selected volume contains no visible directory entries." : "Choose a volume to map its directory allocation." }}</p><div v-if="selected" class="unmapped-volume-stats"><div><span>USED</span><b>{{ volumePercent(selected).toFixed(0) }}%</b></div><div><span>CAPACITY</span><b>{{ selected.total_bytes == null ? "—" : formatBytes(selected.total_bytes) }}</b></div><div><span>AVAILABLE</span><b>{{ selected.available_bytes == null ? "—" : formatBytes(selected.available_bytes) }}</b></div><div class="unmapped-meter" :style="meterStyle(volumePercent(selected))"><i></i></div></div></div>
        </div>
        <div class="tree-keybar"><span>ENTER <b>OPEN</b></span><span>U <b>UP</b></span><span>X <b>MARK</b></span><span>T <b>MODE</b></span><span>P <b>PANEL</b></span><span>R <b>RESCAN</b></span><span class="tree-keybar-status">{{ scanning ? "SCAN IN PROGRESS" : treeRoot ? `${files.toLocaleString()} FILES · ${dirs.toLocaleString()} DIRS` : "AWAITING SCAN" }}</span></div>
      </div>
      <aside v-if="!reviewScreen && showInspector" class="tree-inspector"><div class="inspector-resize" role="separator" aria-orientation="vertical" aria-label="Resize inspection panel" title="Drag to resize, double-click to reset" @pointerdown.prevent="startInspectorResize" @dblclick="inspectorWidth = 286"></div><span class="metric-caption">SELECTION</span><h2>{{ String(selection?.name ?? "No selection") }}</h2><strong class="inspector-size">{{ typeof selection?.bytes === "number" ? formatBytes(selection.bytes) : formatBytes(bytes) }}</strong><p>{{ String(selection?.category ?? "Selected volume") }} · {{ files.toLocaleString() }} files · {{ dirs.toLocaleString() }} folders</p><dl v-if="selection?.path" class="selection-facts"><div v-if="typeof selection.share === 'number'"><dt>SHARE OF SCAN</dt><dd>{{ (selection.share * 100).toFixed(1) }}%</dd></div><div v-if="typeof selection.modified === 'number'"><dt>LAST WRITE</dt><dd>{{ new Date(selection.modified * 1000).toLocaleDateString() }}</dd></div><div v-if="selection.reason"><dt>KIND</dt><dd>{{ String(selection.reason) }}</dd></div></dl><div v-if="unreadable > 0 && isMacOS" class="privacy-hint"><span>{{ unreadable.toLocaleString() }} entries could not be read. Full Disk Access lets Karen measure them.</span><button @click="openFullDiskAccess">OPEN PRIVACY SETTINGS</button></div><div v-if="breadcrumbs.length > 1 && siblings.length" class="sibling-list"><span class="metric-caption">SIBLINGS</span><button v-for="sibling in siblings.slice(0, 8)" :key="sibling.path" :title="sibling.path" @click="requestTreeView(treeRoot, sibling.path)"><em>{{ sibling.name }}</em><span>{{ formatBytes(sibling.bytes) }}</span></button><small v-if="siblings.length > 8">+{{ siblings.length - 8 }} more</small></div><div class="mark-actions"><button :disabled="!selection?.path || selection.path === treeRoot" @click="toggleMark(String(selection?.path))">{{ selection?.marked ? "UNMARK PATH" : "MARK FOR REMOVAL" }}</button><button :disabled="!marks.length" @click="requestReview">REVIEW {{ marks.length }} MARKS</button></div><div v-if="review" class="review-summary"><span>{{ review.targets }} targets · {{ formatBytes(Number(review.reclaim_bytes ?? 0)) }} reclaimable</span><small v-for="blocked in blockedMarks" :key="blocked.path">BLOCKED · {{ blocked.path }} · {{ blocked.reason }}</small><button :disabled="Number(review.targets) === 0" @click="removalConfirm = true">CONTINUE TO REMOVAL</button><button @click="cancelReview">CANCEL REVIEW</button></div><p v-if="removalStatus" class="removal-status">{{ removalStatus }}</p><ul class="marked-list"><li v-for="mark in marks" :key="mark.path"><span>{{ mark.path }}</span><b>{{ formatBytes(mark.bytes) }}</b></li></ul><div class="worth-a-look"><span class="metric-caption">WORTH A LOOK</span><button v-for="finding in findings" :key="finding.path" @click="requestTreeView(treeRoot, finding.path, treeRoot, false)"><span>{{ finding.path.replace(treeRoot, "") }}</span><b>{{ formatBytes(finding.bytes) }}</b><small>{{ finding.reason }}</small></button></div><div v-if="selected" class="inspector-volume"><div class="disk-meter-heading"><span>DISK · {{ selected.device || selected.mount_point }}</span><b>{{ selected.available_bytes == null ? "—" : formatBytes(selected.available_bytes) }} FREE</b></div><div class="disk-meter" :style="meterStyle(volumePercent(selected))"><i></i></div><small>{{ formatBytes((selected.total_bytes ?? 0) - (selected.free_bytes ?? 0)) }} USED <span>{{ selected.total_bytes == null ? "—" : formatBytes(selected.total_bytes) }} TOTAL</span></small></div></aside>
      <section v-if="reviewScreen" class="review-workspace">
        <template v-if="removal">
          <header class="review-heading"><span class="metric-caption">SPACE · {{ removal.phase === "running" ? "REMOVAL IN PROGRESS" : "REMOVAL COMPLETE" }}</span><h1>{{ removal.phase === "running" ? "Removing marked paths" : "Removal finished" }}</h1><p v-if="removal.phase === 'running'">Core is removing each path after re-checking its guards. Closing the window is safe only after this finishes.</p><p v-else>{{ removal.removed }} removed · {{ formatBytes(removal.bytes) }} · {{ removal.failed }} failed</p></header>
          <div class="review-instrument"><div><span>{{ removal.phase === "running" ? "COMPLETED" : "REMOVED" }}</span><strong>{{ removal.phase === "running" ? `${removal.completed} / ${removal.total}` : removal.removed }}</strong></div><div><span>FAILED</span><strong>{{ removal.phase === "running" ? removal.errors.length : removal.failed }}</strong></div><div><span>FREE SPACE CHANGE</span><strong>{{ removalFreeDelta === null ? "MEASURING" : `${removalFreeDelta >= 0 ? "+" : "−"}${formatBytes(Math.abs(removalFreeDelta))}` }}</strong></div></div>
          <div class="review-list"><div class="review-list-heading"><span>PROGRESS</span><span>{{ removal.total ? Math.round(removal.completed / removal.total * 100) : 0 }}%</span></div><div class="removal-progress" role="progressbar" :aria-valuenow="removal.completed" aria-valuemin="0" :aria-valuemax="removal.total"><i :style="{ width: `${removal.total ? removal.completed / removal.total * 100 : 0}%` }"></i></div><div v-for="failure in removal.errors" :key="failure" class="review-blocked"><span>{{ failure }}</span></div></div>
          <div class="review-actions"><button class="review-commit" :disabled="removal.phase === 'running'" @click="finishRemoval">RETURN TO MAP</button></div>
        </template>
        <template v-else>
        <header class="review-heading"><span class="metric-caption">SPACE · DESTRUCTIVE ACTION REVIEW</span><h1>Review marked paths</h1><p>Nothing changes until final confirmation. Core will re-check every path and volume boundary.</p></header>
        <div class="review-instrument"><div><span>TARGETS</span><strong>{{ Number(review?.targets ?? 0) }}</strong></div><div><span>RECLAIM ON THIS VOLUME</span><strong>{{ formatBytes(Number(review?.reclaim_bytes ?? 0)) }}</strong></div><div><span>OTHER VOLUME BYTES</span><strong>{{ formatBytes(Number(review?.foreign_bytes ?? 0)) }}</strong></div></div>
        <div class="review-list"><div class="review-list-heading"><span>PATH / TARGET</span><span>ALLOCATED</span></div><div v-for="mark in reviewMarks" :key="mark.path" class="review-path-row"><span><b>{{ mark.path }}</b><small>{{ mark.is_dir ? "DIRECTORY" : "FILE" }}</small></span><strong>{{ formatBytes(mark.bytes) }}</strong><button class="row-unmark" :aria-label="`Unmark ${mark.path}`" @click="toggleMark(mark.path); requestReview()">UNMARK</button></div><div v-for="blocked in blockedMarks" :key="blocked.path" class="review-blocked"><span>{{ blocked.path }}</span><b>BLOCKED · {{ blocked.reason }}</b></div><p v-if="!reviewMarks.length" class="process-empty">NO MARKED PATHS TO REVIEW</p></div>
        <div class="review-actions"><div class="removal-mode"><span>REMOVAL METHOD</span><button :class="{ active: removalMode === 'trash' }" @click="removalMode = 'trash'">SYSTEM TRASH</button><button :class="{ active: removalMode === 'permanent' }" @click="removalMode = 'permanent'">PERMANENT</button></div><button class="review-back" @click="cancelReview">RETURN TO MAP</button><button class="review-commit" :disabled="Number(review?.targets ?? 0) === 0" @click="removalConfirm = true">CONTINUE TO CONFIRMATION ↗</button></div>
        <div class="review-exports"><button :disabled="!reviewMarks.length" @click="exportMarks('paths', true)">SAVE PATH LIST</button><button :disabled="!reviewMarks.length" @click="exportMarks('prompt', false)">COPY AGENT PROMPT</button><button :disabled="!reviewMarks.length" @click="clearMarks">CLEAR MARKS</button><span v-if="exportStatus" class="export-status">{{ exportStatus }}</span></div>
        </template>
      </section>
      <p v-if="error" class="error" role="alert">{{ error }}</p>
      <div v-if="removalConfirm" class="removal-shade" role="dialog" aria-modal="true" aria-labelledby="removal-title"><section class="removal-confirm"><span class="metric-caption">DESTRUCTIVE OPERATION</span><h2 id="removal-title">Confirm marked paths</h2><p>{{ review?.targets }} targets · {{ formatBytes(Number(review?.reclaim_bytes ?? 0)) }} expected volume recovery</p><label><input v-model="removalMode" type="radio" value="trash"> Move to system Trash</label><label><input v-model="removalMode" type="radio" value="permanent"> Delete permanently</label><div class="confirm-actions"><button @click="cancelReview">CANCEL</button><button @click="confirmRemoval">CONFIRM {{ removalMode === "trash" ? "TRASH" : "PERMANENT DELETE" }}</button></div></section></div>
    </section>
    <div v-if="helpOpen" class="removal-shade" role="dialog" aria-modal="true" aria-labelledby="help-title" @click.self="helpOpen = false"><section class="help-sheet"><header><span class="metric-caption">SPACE · KEYBOARD AND MOUSE</span><h2 id="help-title">Controls</h2></header><dl><template v-for="[keys, action] in helpRows" :key="keys"><dt>{{ keys }}</dt><dd>{{ action }}</dd></template></dl><button class="review-back" @click="helpOpen = false">CLOSE · ESC</button></section></div>
    <aside v-if="hoveredTile?.tile.path && tab === 'disk' && !reviewScreen" class="tree-tooltip" :style="{ left: `${hoveredTile.x}px`, top: `${hoveredTile.y}px` }" aria-live="polite">
      <div class="tree-tooltip-title"><i>{{ hoveredTile.tile.is_dir ? "▰" : "▪" }}</i><b>{{ hoveredTile.tile.name }}</b></div>
      <small>{{ hoveredTile.tile.path }}</small>
      <div class="tree-tooltip-reading"><strong>{{ hoveredTile.tile.bytes === null ? "MERGED ITEMS" : formatBytes(hoveredTile.tile.bytes) }}</strong><i :style="{ width: `${hoveredShare}%` }"></i><span>{{ hoveredShare.toFixed(1) }}%</span></div>
      <span>{{ hoveredTile.tile.files?.toLocaleString() ?? "—" }} FILES · {{ hoveredTile.tile.dirs?.toLocaleString() ?? "—" }} DIRS · {{ formatBytes(hoveredTile.tile.direct_bytes ?? 0) }} DIRECT</span>
      <span v-if="hoveredTile.tile.name.startsWith('.')">HIDDEN</span>
      <span v-if="isHoveredMarked">MARKED FOR REMOVAL</span>
      <span v-else-if="isHoveredCovered">INSIDE MARKED {{ coveredMarkName }}</span>
      <span class="tree-tooltip-keys">{{ hoveredTile.tile.is_dir ? "SPACE MARK · ENTER OPEN" : "SPACE MARK" }}</span>
    </aside>
  </main>
  <Teleport to="body">
    <div v-if="volumeMenuOpen" class="volume-options" :class="{ above: volumeMenuAbove }" :style="volumeMenuStyle" role="listbox" aria-label="Select volume">
      <button v-for="volume in volumes" :key="volume.mount_point" role="option" :aria-selected="volume.mount_point === selectedVolume" :class="{ chosen: volume.mount_point === selectedVolume }" @click="selectVolume(volume.mount_point)"><i class="drive-lamp"></i><span><b>{{ volume.mount_point }}</b><small>{{ volume.device || "LOCAL STORAGE" }}</small></span><em>{{ volumePercent(volume).toFixed(0) }}% USED</em><i class="option-check">{{ volume.mount_point === selectedVolume ? "✓" : "" }}</i></button>
      <p v-if="!volumes.length">No mounted volumes</p>
    </div>
  </Teleport>
</template>
