<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { formatBytes, meterStyle, volumePercent } from "../format";
import { useI18n } from "../i18n";
import { categoryColor, drawTreemap, tileAt, type TreeTile } from "../treemap";
import type { BlockedMark, Crumb, DiskMark, Finding, PowerPreset, RemovalState, Sibling, TreeMode, Volume } from "../types";

const { t } = useI18n();

const props = defineProps<{
  selectedVolume: string;
  volumes: Volume[];
  treeRoot: string;
  tiles: TreeTile[];
  files: number;
  dirs: number;
  bytes: number;
  scanProgress: { files: number; dirs: number; bytes: number };
  error: string;
  scanning: boolean;
  previewing: boolean;
  workers: number | null;
  power: PowerPreset;
  unreadable: number;
  exportStatus: string;
  removal: RemovalState | null;
  siblings: Sibling[];
  findings: Finding[];
  marks: DiskMark[];
  selection: Record<string, unknown> | null;
  review: Record<string, unknown> | null;
  reviewScreen: boolean;
  breadcrumbs: Crumb[];
  removalConfirm: boolean;
  removalMode: "trash" | "permanent";
  removalStatus: string;
  isMacOS: boolean;
  treeMode: TreeMode;
  treeDepth: number;
  includeHidden: boolean;
  apparentSize: boolean;
}>();

const emit = defineEmits<{
  scan: [path?: string];
  selectVolume: [path: string];
  requestTreeView: [params: { path: string; selectedPath?: string; root?: string; open?: boolean; keepView?: boolean; metric?: "size" | "files"; depth?: number; width?: number; height?: number }];
  toggleMark: [path: string];
  requestReview: [];
  cancelReview: [];
  confirmRemoval: [mode: "trash" | "permanent"];
  finishRemoval: [];
  exportMarks: [format: "paths" | "prompt", save: boolean];
  clearMarks: [];
  revealPath: [path: string | null];
  chooseDirectory: [];
  openFullDiskAccess: [];
  setPower: [preset: PowerPreset];
  closeWindow: [];
  "update:treeMode": [mode: TreeMode];
  "update:treeDepth": [depth: number];
  "update:includeHidden": [value: boolean];
  "update:apparentSize": [value: boolean];
  "update:reviewScreen": [value: boolean];
  "update:removalConfirm": [value: boolean];
  "update:removalMode": [mode: "trash" | "permanent"];
}>();

const treeModes: readonly TreeMode[] = ["size", "files", "age"] as const;
const powerPresets: { id: PowerPreset; label: string; note: string }[] = [
  { id: "miser", label: "Miser", note: "fewest workers, lowest battery use" },
  { id: "balanced", label: "Balanced", note: "default scan speed" },
  { id: "aggressive", label: "Aggressive", note: "more workers, faster scan" },
  { id: "drain", label: "Drain My Battery", note: "every available worker" },
];

const helpRows: [string, string][] = [
  ["Click / double-click tile", "Select / open directory (double-click)"],
  ["Right-click tile", "Path actions: mark or unmark, reveal in the file manager"],
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

const vimArrows: Record<string, string> = { h: "ArrowLeft", j: "ArrowDown", k: "ArrowUp", l: "ArrowRight" };

const treemapCanvas = ref<HTMLCanvasElement | null>(null);
const treemapSize = ref({ width: 1200, height: 680 });
const volumeMenuOpen = ref(false);
const volumePickerTrigger = ref<HTMLButtonElement | null>(null);
const volumeMenuStyle = ref<Record<string, string>>({});
const volumeMenuAbove = ref(false);
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
let pointerLast = false;
const inspectorOpen = ref(true);
const inspectorWidth = ref(286);
const viewportWidth = ref(window.innerWidth);
const hoveredTile = ref<{ tile: TreeTile; x: number; y: number } | null>(null);
let treemapResizeObserver: ResizeObserver | undefined;

const showInspector = computed(() => inspectorOpen.value && viewportWidth.value >= 832);
const selected = computed(() => props.volumes.find((volume) => volume.mount_point === props.selectedVolume));
const visibleCrumbs = computed(() => {
  const all = props.breadcrumbs.map((crumb, index) => ({ crumb, index }));
  if (all.length <= 5) return all.map((item) => ({ ...item, gap: false }));
  return [{ ...all[0], gap: false }, { crumb: all[0].crumb, index: -1, gap: true }, ...all.slice(-3).map((item) => ({ ...item, gap: false }))];
});
const workerLabel = computed(() => props.workers === null ? "" : ` · ${props.workers} WORKERS`);
const powerLabel = computed(() => powerPresets.find((preset) => preset.id === props.power)?.label ?? "Balanced");
const removalFreeDelta = computed(() => props.removal?.freeBefore != null && props.removal.freeAfter != null
  ? props.removal.freeAfter - props.removal.freeBefore
  : null);
const reviewMarks = computed(() => Array.isArray(props.review?.marks)
  ? props.review.marks as DiskMark[]
  : props.marks);
const blockedMarks = computed(() => Array.isArray(props.review?.blocked)
  ? props.review.blocked as BlockedMark[]
  : []);

const hoveredShare = computed(() => {
  const tile = hoveredTile.value?.tile;
  if (!tile?.parent_bytes) return 100;
  return Math.min(100, (tile.bytes ?? 0) / tile.parent_bytes * 100);
});
const isHoveredMarked = computed(() => {
  const path = hoveredTile.value?.tile.path;
  return Boolean(path && props.marks.some((mark) => mark.path === path));
});
function coveredMarkFor(path: string | null | undefined): string {
  if (!path) return "";
  const separator = path.includes("\\") ? "\\" : "/";
  const mark = props.marks.find((item) => path.startsWith(`${item.path}${separator}`));
  return mark?.path.split(separator).at(-1) ?? "";
}
const coveredMarkName = computed(() => coveredMarkFor(hoveredTile.value?.tile.path));
const pathMenu = ref<{ path: string; x: number; y: number } | null>(null);
const pathMenuEl = ref<HTMLElement | null>(null);
const pathMenuMarked = computed(() => Boolean(pathMenu.value && props.marks.some((mark) => mark.path === pathMenu.value?.path)));
const pathMenuCovered = computed(() => coveredMarkFor(pathMenu.value?.path));
const pathMenuStyle = computed(() => {
  const menu = pathMenu.value!;
  const width = 240;
  const height = 100;
  const edge = 8;
  return {
    left: `${Math.max(edge, Math.min(menu.x, window.innerWidth - width - edge))}px`,
    top: `${Math.max(edge, Math.min(menu.y, window.innerHeight - height - edge))}px`,
  };
});
const isHoveredCovered = computed(() => coveredMarkName.value.length > 0);

function requestTree(
  path: string,
  selectedPath = path,
  root = props.treeRoot || props.selectedVolume,
  open = true,
  keepView = false,
): void {
  emit("requestTreeView", {
    path,
    selectedPath,
    root,
    open,
    keepView,
    metric: props.treeMode === "files" ? "files" : "size",
    depth: props.treeDepth,
    width: treemapSize.value.width,
    height: treemapSize.value.height,
  });
}

function selectTile(tile: TreeTile, event: MouseEvent): void {
  if (!tile.path) return;
  // Only the second click of a double-click drills in; a third rapid click would open whatever now sits under the pointer.
  const open = tile.is_dir && event.detail === 2;
  requestTree(props.treeRoot, tile.path, props.treeRoot, open, !open);
}

function setTreeMode(mode: TreeMode): void {
  emit("update:treeMode", mode);
  const directory = props.breadcrumbs.at(-1)?.path ?? props.treeRoot;
  if (directory) {
    emit("requestTreeView", {
      path: directory,
      selectedPath: directory,
      root: props.treeRoot,
      open: true,
      metric: mode === "files" ? "files" : "size",
      depth: props.treeDepth,
      width: treemapSize.value.width,
      height: treemapSize.value.height,
    });
  }
}

function setTreeDepth(step: number): void {
  const nextDepth = Math.max(1, Math.min(6, props.treeDepth + step));
  emit("update:treeDepth", nextDepth);
  const directory = props.breadcrumbs.at(-1)?.path ?? props.treeRoot;
  if (directory) {
    emit("requestTreeView", {
      path: directory,
      selectedPath: directory,
      root: props.treeRoot,
      open: true,
      metric: props.treeMode === "files" ? "files" : "size",
      depth: nextDepth,
      width: treemapSize.value.width,
      height: treemapSize.value.height,
    });
  }
}

function rescanWithFilter(): void {
  if (props.treeRoot || props.selectedVolume) {
    emit("scan", props.treeRoot || props.selectedVolume);
  }
}

function setPower(preset: PowerPreset): void {
  powerMenuOpen.value = false;
  emit("setPower", preset);
}

function goHistory(step: number): void {
  const target = history.value[historyIndex.value + step];
  if (!target) return;
  historyIndex.value += step;
  replayingHistory = true;
  requestTree(target.path, target.path, target.root, true);
}

function openSibling(sibling: Sibling): void {
  crumbMenu.value = null;
  requestTree(sibling.is_dir ? sibling.path : props.treeRoot, sibling.path, props.treeRoot, sibling.is_dir);
}

function widenToVolume(): void {
  if (props.selectedVolume) requestTree(props.selectedVolume, props.selectedVolume, props.selectedVolume, true);
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
  closePathMenu();
  const point = treeCoords(event);
  if (!point) return;
  const current = zoom.value;
  if (event.shiftKey) {
    zoom.value = clampZoom(current.scale, current.x - (event.deltaY || event.deltaX), current.y);
    return;
  }
  const scale = Math.max(1, Math.min(8, current.scale * Math.exp(-event.deltaY * 0.0015)));
  if (scale === current.scale && event.deltaY > 0 && props.breadcrumbs.length > 1) {
    goToParent();
    return;
  }
  zoom.value = clampZoom(scale, point.px - (point.px - current.x) * scale / current.scale, point.py - (point.py - current.y) * scale / current.scale);
  const tile = tileAt(props.tiles, point.x, point.y);
  if (scale >= 3 && tile?.is_dir && tile.path) requestTree(tile.path, tile.path, props.treeRoot, true);
}

function actionPath(): string | null {
  const pointed = hoveredTile.value?.tile.path;
  if (pointerLast && pointed) return pointed;
  return typeof props.selection?.path === "string" ? props.selection.path : null;
}

function goToParent(): void {
  if (props.breadcrumbs.length <= 1) return;
  const parent = props.breadcrumbs[props.breadcrumbs.length - 2];
  requestTree(parent.path, parent.path, props.treeRoot, true);
}

function stepSibling(step: number): void {
  const list = props.siblings;
  const at = list.findIndex((sibling) => sibling.path === props.selection?.path);
  const next = list[(at + step + list.length) % list.length];
  if (next) requestTree(props.treeRoot, next.path, props.treeRoot, false, true);
}

function handleDiskKeyboard(event: KeyboardEvent): void {
  if (props.reviewScreen || event.metaKey || event.ctrlKey || event.altKey) return;
  const target = event.target;
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return;
  if (target instanceof Element && target.closest("button:not(.tree-tile)")) return;

  if (helpOpen.value || pathMenu.value) return;
  const key = vimArrows[event.key] ?? event.key;
  if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Tab"].includes(key)) pointerLast = false;
  if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(key)) {
    const current = props.tiles.find((tile) => tile.path === props.selection?.path);
    const candidates = props.tiles.filter((tile) => tile.path && tile.path !== current?.path);
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
      requestTree(props.treeRoot, next.path, props.treeRoot, false, true);
    }
    return;
  }

  const targetPath = actionPath();
  if (event.key === "Enter" && targetPath) {
    const targetTile = props.tiles.find((tile) => tile.path === targetPath);
    if (targetTile?.is_dir) {
      event.preventDefault();
      requestTree(props.treeRoot, targetPath, props.treeRoot, true);
    }
  } else if ((event.code === "Space" || event.key === "x") && targetPath) {
    event.preventDefault();
    emit("toggleMark", targetPath);
  } else if (event.key === "/" || event.key === "s") {
    event.preventDefault();
    openFilter();
  } else if (event.key === "?") {
    helpOpen.value = true;
  } else if (event.key === "g") {
    widenToVolume();
  } else if (event.key === "o") {
    emit("revealPath", targetPath);
  } else if (event.key === "c" && props.marks.length) {
    emit("requestReview");
  } else if (event.key === "q") {
    emit("closeWindow");
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
  } else if (event.key === "d" && !props.scanning) {
    emit("update:apparentSize", !props.apparentSize);
    rescanWithFilter();
  } else if (event.key === "i" && !props.scanning) {
    emit("update:includeHidden", !props.includeHidden);
    rescanWithFilter();
  } else if (event.key === "[") {
    event.preventDefault();
    setTreeDepth(-1);
  } else if (event.key === "]") {
    event.preventDefault();
    setTreeDepth(1);
  } else if (event.key.toLowerCase() === "t") {
    const index = treeModes.indexOf(props.treeMode);
    setTreeMode(treeModes[(index + 1) % treeModes.length]);
  } else if (event.key.toLowerCase() === "r" && !props.scanning) {
    emit("scan", props.treeRoot || props.selectedVolume);
  }
}

function handleDiskHistoryKeys(event: KeyboardEvent): void {
  const command = event.metaKey || event.ctrlKey;
  if (command && event.key.toLowerCase() === "o") {
    event.preventDefault();
    emit("chooseDirectory");
  } else if ((event.altKey && event.key === "ArrowLeft") || (event.metaKey && event.key === "[")) {
    event.preventDefault();
    goHistory(-1);
  } else if ((event.altKey && event.key === "ArrowRight") || (event.metaKey && event.key === "]")) {
    event.preventDefault();
    goHistory(1);
  } else if (event.key === "Escape" && helpOpen.value) {
    helpOpen.value = false;
  }
}

function handleHistoryButtons(event: MouseEvent): void {
  if (event.button !== 3 && event.button !== 4) return;
  event.preventDefault();
  goHistory(event.button === 3 ? -1 : 1);
}

function closeVolumeMenu(event: PointerEvent): void {
  const target = event.target;
  if (target instanceof Element && !target.closest(".volume-picker, .volume-options")) {
    volumeMenuOpen.value = false;
  }
  if (!(target instanceof Element && target.closest(".tree-context-menu"))) closePathMenu();
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
  const desiredHeight = Math.min(props.volumes.length * 48 + 10, window.innerHeight - edge * 2);
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

function onSelectVolume(path: string): void {
  volumeMenuOpen.value = false;
  emit("selectVolume", path);
}

function closeVolumeMenuOnEscape(event: KeyboardEvent): void {
  if (event.key !== "Escape") return;
  if (pathMenu.value) {
    // handleDiskKeyboard is registered after this listener and would otherwise run its Escape branch (go to parent).
    event.stopImmediatePropagation();
    closePathMenu();
    return;
  }
  if (props.removalConfirm || props.review) {
    emit("cancelReview");
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
  const tile = point ? tileAt(props.tiles, point.x, point.y) : undefined;
  if (!tile) {
    hoveredTile.value = null;
    return;
  }
  showTileTooltip(tile, event);
}

function clickTreeCanvas(event: MouseEvent): void {
  const canvas = treemapCanvas.value;
  if (!canvas) return;
  if (event.type === "auxclick" && event.button !== 1) return;
  const point = treeCoords(event);
  const tile = point ? tileAt(props.tiles, point.x, point.y) : undefined;
  if (!tile?.path) return;
  if (event.ctrlKey || event.metaKey || event.button === 1) {
    emit("toggleMark", tile.path);
    return;
  }
  selectTile(tile, event);
}

function closePathMenu(): void {
  // The focused menu button is about to leave the DOM; hand focus back so keyboard shortcuts keep working.
  if (pathMenuEl.value?.contains(document.activeElement)) focusTreemap();
  pathMenu.value = null;
}

function runPathMenu(action: "toggleMark" | "revealPath"): void {
  const path = pathMenu.value!.path;
  closePathMenu();
  if (action === "toggleMark") emit("toggleMark", path);
  else emit("revealPath", path);
}

function keyPathMenu(event: KeyboardEvent): void {
  if (event.key === "Tab") {
    closePathMenu();
    return;
  }
  const keys = ["ArrowDown", "ArrowUp", "Home", "End"];
  if (!keys.includes(event.key)) return;
  event.preventDefault();
  // Reveal is always enabled, so there is at least one focusable item.
  const buttons = [...pathMenuEl.value!.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
  const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
  const next = event.key === "Home" ? 0
    : event.key === "End" ? buttons.length - 1
    : (at + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
  buttons[next].focus();
}

function contextTree(event: MouseEvent): void {
  // This handler is bound to the canvas, so it is mounted.
  const point = treeCoords(event)!;
  let tile = tileAt(props.tiles, point.x, point.y);
  let x = event.clientX;
  let y = event.clientY;
  const bounds = treemapCanvas.value!.getBoundingClientRect();
  if (!tile && event.button !== 2) {
    // A keyboard-invoked menu carries no pointer position, so anchor it to the selected tile.
    tile = props.tiles.find((item) => item.path === props.selection?.path);
    if (tile) {
      x = bounds.left + ((tile.x + tile.width / 2) * zoom.value.scale + zoom.value.x) * bounds.width / treemapSize.value.width;
      y = bounds.top + ((tile.y + tile.height / 2) * zoom.value.scale + zoom.value.y) * bounds.height / treemapSize.value.height;
    }
  }
  if (!tile?.path) return;
  requestTree(props.treeRoot, tile.path, props.treeRoot, false, true);
  hoveredTile.value = null;
  pathMenu.value = { path: tile.path, x, y };
  void nextTick(() => pathMenuEl.value?.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus());
}

function drawCurrentTreemap(): void {
  if (!treemapCanvas.value) return;
  drawTreemap({
    canvas: treemapCanvas.value,
    tiles: props.tiles,
    marks: props.marks,
    selectedPath: typeof props.selection?.path === "string" ? props.selection.path : null,
    hoveredPath: hoveredTile.value?.tile.path ?? null,
    mode: props.treeMode,
    width: treemapSize.value.width,
    height: treemapSize.value.height,
    filter: filterText.value,
    zoom: zoom.value,
  });
}

async function observeTreemap(): Promise<void> {
  await nextTick();
  const canvas = treemapCanvas.value;
  if (!canvas || typeof ResizeObserver === "undefined") return;
  treemapResizeObserver?.disconnect();
  treemapResizeObserver = new ResizeObserver(([entry]) => {
    const width = Math.max(320, Math.round(entry.contentRect.width));
    const height = Math.max(240, Math.round(entry.contentRect.height));
    if (width === treemapSize.value.width && height === treemapSize.value.height) return;
    treemapSize.value = { width, height };
    const directory = props.breadcrumbs.at(-1)?.path ?? props.treeRoot;
    if (directory) {
      emit("requestTreeView", {
        path: directory,
        selectedPath: directory,
        root: props.treeRoot,
        open: true,
        metric: props.treeMode === "files" ? "files" : "size",
        depth: props.treeDepth,
        width,
        height,
      });
    }
  });
  treemapResizeObserver.observe(canvas);
}

function syncViewportWidth(): void {
  viewportWidth.value = window.innerWidth;
}

watch(
  [() => props.tiles, () => props.marks, () => props.selection, hoveredTile, () => props.treeMode, treemapSize, filterText, zoom],
  drawCurrentTreemap,
  { deep: true },
);

watch(() => props.tiles, (tiles) => {
  // The right-click that opens the menu also requests the tree, so only close once its path is gone.
  if (pathMenu.value && !tiles.some((tile) => tile.path === pathMenu.value?.path)) closePathMenu();
});

watch(() => props.reviewScreen || props.removalConfirm, (open) => {
  if (open) closePathMenu();
});

watch(() => props.reviewScreen, async (open) => {
  if (open) return;
  await observeTreemap();
  drawCurrentTreemap();
});

watch(
  () => props.breadcrumbs,
  (crumbs) => {
    const viewDir = crumbs.at(-1)?.path ?? "";
    if (viewDir && viewDir !== lastViewDir) {
      lastViewDir = viewDir;
      closePathMenu();
      zoom.value = { scale: 1, x: 0, y: 0 };
      if (!replayingHistory) {
        history.value = [...history.value.slice(0, historyIndex.value + 1), { root: props.treeRoot, path: viewDir }];
        historyIndex.value = history.value.length - 1;
      }
    }
    replayingHistory = false;
  },
  { deep: true, immediate: true },
);

onMounted(async () => {
  window.addEventListener("pointerdown", closeVolumeMenu);
  window.addEventListener("resize", closePathMenu);
  window.addEventListener("blur", closePathMenu);
  window.addEventListener("resize", positionVolumeMenu);
  window.addEventListener("resize", syncViewportWidth);
  window.addEventListener("scroll", positionVolumeMenu, true);
  window.addEventListener("keydown", closeVolumeMenuOnEscape);
  window.addEventListener("keydown", handleDiskKeyboard);
  window.addEventListener("keydown", handleDiskHistoryKeys);
  window.addEventListener("mouseup", handleHistoryButtons);
  await observeTreemap();
});

onBeforeUnmount(() => {
  treemapResizeObserver?.disconnect();
  window.removeEventListener("pointerdown", closeVolumeMenu);
  window.removeEventListener("resize", closePathMenu);
  window.removeEventListener("blur", closePathMenu);
  window.removeEventListener("resize", positionVolumeMenu);
  window.removeEventListener("scroll", positionVolumeMenu, true);
  window.removeEventListener("keydown", closeVolumeMenuOnEscape);
  window.removeEventListener("keydown", handleDiskKeyboard);
  window.removeEventListener("keydown", handleDiskHistoryKeys);
  window.removeEventListener("mouseup", handleHistoryButtons);
  window.removeEventListener("resize", syncViewportWidth);
});
</script>

<template>
  <section class="view disk-view" :class="{ 'no-inspector': !showInspector }" :style="{ '--inspector-w': `${inspectorWidth}px` }">
    <div v-if="!reviewScreen" class="tree-console">
      <div class="tree-toolbar">
        <div class="tree-location">
          <div class="volume-picker">
            <button ref="volumePickerTrigger" class="volume-picker-trigger" aria-haspopup="listbox" :aria-expanded="volumeMenuOpen" @click="toggleVolumeMenu">
              <i class="drive-lamp"></i><strong>{{ selectedVolume || "NO VOLUME" }}</strong><small>{{ selected?.device || "LOCAL STORAGE" }}</small><i class="picker-chevron" :class="{ open: volumeMenuOpen }"></i>
            </button>
          </div>
          <span class="location-lamp"></span>
          <div class="history-buttons">
            <button aria-label="History back" :title="history[historyIndex - 1]?.path ?? 'Nothing earlier'" :disabled="historyIndex <= 0" @click="goHistory(-1)">‹</button>
            <button aria-label="History forward" :title="history[historyIndex + 1]?.path ?? 'Nothing later'" :disabled="historyIndex >= history.length - 1" @click="goHistory(1)">›</button>
          </div>
          <template v-for="item in visibleCrumbs" :key="item.gap ? 'gap' : item.crumb.path">
            <span v-if="item.gap" class="tree-crumb-gap" aria-hidden="true">…</span>
            <span v-else class="tree-crumb-wrap">
              <button class="tree-crumb" :title="item.crumb.path" @click="requestTree(item.crumb.path, item.crumb.path, treeRoot, true)">{{ item.crumb.label }}</button>
              <button v-if="(item.crumb.siblings?.length ?? 0) > 1" class="crumb-siblings" :aria-label="`Siblings of ${item.crumb.label}`" :aria-expanded="crumbMenu === item.index" @click.stop="crumbMenu = crumbMenu === item.index ? null : item.index">▾</button>
              <div v-if="crumbMenu === item.index" class="crumb-menu" role="listbox" :aria-label="`Siblings of ${item.crumb.label}`">
                <button v-for="sibling in item.crumb.siblings" :key="sibling.path" role="option" :class="{ chosen: sibling.path === item.crumb.path }" @click="openSibling(sibling)">
                  <i class="crumb-dot" :style="{ background: categoryColor(sibling.category ?? '') }"></i>
                  <span class="crumb-name">{{ sibling.name }}</span>
                  <i class="crumb-bar"><b :style="{ width: `${Math.max(2, sibling.bytes / (item.crumb.siblings?.[0]?.bytes || 1) * 100)}%` }"></b></i>
                  <span class="crumb-size">{{ formatBytes(sibling.bytes) }}</span>
                </button>
              </div>
            </span>
          </template>
        </div>
        <div class="tree-actions">
          <div class="power-picker">
            <button class="power-trigger" aria-haspopup="listbox" :aria-expanded="powerMenuOpen" title="Power Efficiency: scan worker preset" @click="powerMenuOpen = !powerMenuOpen">
              POWER · {{ powerLabel.toUpperCase() }}{{ workerLabel }}
            </button>
            <div v-if="powerMenuOpen" class="power-menu" role="listbox" aria-label="Power Efficiency">
              <button v-for="preset in powerPresets" :key="preset.id" role="option" :class="{ chosen: power === preset.id }" @click="setPower(preset.id)">
                <b>{{ preset.label }}</b><small>{{ preset.note }}</small>
              </button>
            </div>
          </div>
          <span v-if="previewing" class="tree-preview-badge">SCANNING · PREVIOUS MAP RETAINED</span>
          <button class="map-button" :disabled="scanning || !selectedVolume" @click="emit('scan', treeRoot || selectedVolume)">
            {{ scanning ? t("space.scanning") : treeRoot ? t("space.rescan") : t("space.scanRoot") }}
          </button>
        </div>
      </div>
      <div class="tree-controls" @click="($event.target as HTMLElement).closest('button, label') && focusTreemap()">
        <div class="tree-mode" role="group" aria-label="Treemap mode">
          <button v-for="mode in treeModes" :key="mode" :class="{ active: treeMode === mode }" @click="setTreeMode(mode)">{{ mode.toUpperCase() }}</button>
        </div>
        <label class="tree-check">
          <input :checked="includeHidden" type="checkbox" :disabled="scanning" @change="emit('update:includeHidden', ($event.target as HTMLInputElement).checked); rescanWithFilter()">
          <span>HIDDEN FILES</span>
        </label>
        <label class="tree-check">
          <input :checked="apparentSize" type="checkbox" :disabled="scanning" @change="emit('update:apparentSize', ($event.target as HTMLInputElement).checked); rescanWithFilter()">
          <span>{{ t("space.apparent") }}</span>
        </label>
        <label v-show="filterOpen || filterText" class="tree-filter">
          <span>FILTER</span>
          <input ref="filterInput" v-model="filterText" type="text" placeholder="name contains…" aria-label="Filter tiles by name" @keydown.enter.prevent="closeFilter(false)" @keydown.esc.stop.prevent="closeFilter(true)">
        </label>
        <div class="depth-control">
          <span>{{ t("space.depthLimit") }} {{ treeDepth }}</span>
          <button aria-label="Decrease treemap depth" :disabled="treeDepth <= 1" @click="setTreeDepth(-1)">−</button>
          <button aria-label="Increase treemap depth" :disabled="treeDepth >= 6" @click="setTreeDepth(1)">+</button>
        </div>
      </div>
      <div class="tree-legend" :class="{ 'age-mode': treeMode === 'age' }">
        <span v-if="treeRoot" class="tree-count">
          {{ previewing ? `SCAN ${scanProgress.files.toLocaleString()} FILES` : formatBytes(bytes) }} <i>·</i> {{ files.toLocaleString() }} files <i>·</i> {{ dirs.toLocaleString() }} dirs
          <template v-if="unreadable"> <i>·</i> {{ unreadable.toLocaleString() }} unreadable</template>
          <template v-if="filterText"> <i>·</i> filter “{{ filterText }}”</template>
        </span>
        <span class="age-legend"><i></i> THIS WEEK <i></i> THIS MONTH <i></i> SIX MONTHS <i></i> THIS YEAR <i></i> OLDER</span>
        <span><i class="legend-code"></i> CODE</span><span><i class="legend-agent"></i> AGENT SCRATCH</span><span><i class="legend-tool"></i> TOOLCHAINS</span><span><i class="legend-synced"></i> SYNCED</span><span><i class="legend-git"></i> GIT</span><span><i class="legend-media"></i> MEDIA</span><span><i class="legend-documents"></i> DOCUMENTS</span><span><i class="legend-cache"></i> CACHE</span><span><i class="legend-other"></i> OTHER</span>
      </div>
      <div class="treemap-frame" :class="{ mapped: tiles.length > 0 }">
        <canvas
          v-show="tiles.length"
          ref="treemapCanvas"
          class="treemap-canvas"
          tabindex="0"
          role="img"
          aria-label="Disk usage treemap. Use arrow keys to select a tile, Enter to open, Space to mark, and the context menu key for path actions."
          @pointermove="updateTreePointer"
          @pointerleave="hoveredTile = null"
          @click="clickTreeCanvas"
          @auxclick.prevent="clickTreeCanvas"
          @contextmenu.prevent="contextTree"
          @wheel.prevent="wheelTree"
        ></canvas>
        <div v-if="!tiles.length" class="tree-empty">
          <span class="empty-state-label">{{ scanning ? "MAPPING STORAGE" : treeRoot ? "NO DIRECTORY ENTRIES" : "VOLUME NOT MAPPED" }}</span>
          <p>{{ scanning ? `${scanProgress.files.toLocaleString()} files · ${formatBytes(scanProgress.bytes)} inspected${previewing ? " · previous completed map retained" : ""}` : treeRoot ? "Selected volume contains no visible directory entries." : "Choose a volume to map its directory allocation." }}</p>
          <div v-if="selected" class="unmapped-volume-stats">
            <div><span>USED</span><b>{{ volumePercent(selected).toFixed(0) }}%</b></div>
            <div><span>CAPACITY</span><b>{{ selected.total_bytes == null ? "—" : formatBytes(selected.total_bytes) }}</b></div>
            <div><span>AVAILABLE</span><b>{{ selected.available_bytes == null ? "—" : formatBytes(selected.available_bytes) }}</b></div>
            <div class="unmapped-meter" :style="meterStyle(volumePercent(selected))"><i></i></div>
          </div>
        </div>
      </div>
      <div class="tree-keybar">
        <span>ENTER <b>OPEN</b></span><span>U <b>UP</b></span><span>X <b>MARK</b></span><span>T <b>MODE</b></span><span>P <b>PANEL</b></span><span>R <b>RESCAN</b></span>
        <span class="tree-keybar-status">{{ scanning ? "SCAN IN PROGRESS" : treeRoot ? `${files.toLocaleString()} FILES · ${dirs.toLocaleString()} DIRS` : "AWAITING SCAN" }}</span>
      </div>
    </div>
    <aside v-if="!reviewScreen && showInspector" class="tree-inspector">
      <div class="inspector-resize" role="separator" aria-orientation="vertical" aria-label="Resize inspection panel" title="Drag to resize, double-click to reset" @pointerdown.prevent="startInspectorResize" @dblclick="inspectorWidth = 286"></div>
      <span class="metric-caption">SELECTION</span>
      <h2>{{ String(selection?.name ?? "No selection") }}</h2>
      <strong class="inspector-size">{{ typeof selection?.bytes === "number" ? formatBytes(selection.bytes) : formatBytes(bytes) }}</strong>
      <p>{{ String(selection?.category ?? "Selected volume") }} · {{ files.toLocaleString() }} files · {{ dirs.toLocaleString() }} folders</p>
      <dl v-if="selection?.path" class="selection-facts">
        <div v-if="typeof selection.share === 'number'"><dt>SHARE OF SCAN</dt><dd>{{ (selection.share * 100).toFixed(1) }}%</dd></div>
        <div v-if="typeof selection.modified === 'number'"><dt>LAST WRITE</dt><dd>{{ new Date(selection.modified * 1000).toLocaleDateString() }}</dd></div>
        <div v-if="selection.reason"><dt>KIND</dt><dd>{{ String(selection.reason) }}</dd></div>
      </dl>
      <div v-if="unreadable > 0 && isMacOS" class="privacy-hint">
        <span>{{ unreadable.toLocaleString() }} entries could not be read. Full Disk Access lets Karen measure them.</span>
        <button @click="emit('openFullDiskAccess')">OPEN PRIVACY SETTINGS</button>
      </div>
      <div v-if="breadcrumbs.length > 1 && siblings.length" class="sibling-list">
        <span class="metric-caption">SIBLINGS</span>
        <button v-for="sibling in siblings.slice(0, 8)" :key="sibling.path" :title="sibling.path" @click="requestTree(treeRoot, sibling.path)">
          <em>{{ sibling.name }}</em><span>{{ formatBytes(sibling.bytes) }}</span>
        </button>
        <small v-if="siblings.length > 8">+{{ siblings.length - 8 }} more</small>
      </div>
      <div class="mark-actions">
        <button :disabled="!selection?.path || selection.path === treeRoot" @click="emit('toggleMark', String(selection?.path))">
          {{ selection?.marked ? "UNMARK PATH" : "MARK FOR REMOVAL" }}
        </button>
        <button :disabled="!marks.length" @click="emit('requestReview')">REVIEW {{ marks.length }} MARKS</button>
      </div>
      <div v-if="review" class="review-summary">
        <span>{{ review.targets }} targets · {{ formatBytes(Number(review.reclaim_bytes ?? 0)) }} reclaimable</span>
        <small v-for="blocked in blockedMarks" :key="blocked.path">BLOCKED · {{ blocked.path }} · {{ blocked.reason }}</small>
        <button :disabled="Number(review.targets) === 0" @click="emit('update:removalConfirm', true)">CONTINUE TO REMOVAL</button>
        <button @click="emit('cancelReview')">CANCEL REVIEW</button>
      </div>
      <p v-if="removalStatus" class="removal-status">{{ removalStatus }}</p>
      <ul class="marked-list">
        <li v-for="mark in marks" :key="mark.path"><span>{{ mark.path }}</span><b>{{ formatBytes(mark.bytes) }}</b></li>
      </ul>
      <div class="worth-a-look">
        <span class="metric-caption">WORTH A LOOK</span>
        <button v-for="finding in findings" :key="finding.path" @click="requestTree(treeRoot, finding.path, treeRoot, false)">
          <span>{{ finding.path.replace(treeRoot, "") }}</span><b>{{ formatBytes(finding.bytes) }}</b><small>{{ finding.reason }}</small>
        </button>
      </div>
      <div v-if="selected" class="inspector-volume">
        <div class="disk-meter-heading"><span>DISK · {{ selected.device || selected.mount_point }}</span><b>{{ selected.available_bytes == null ? "—" : formatBytes(selected.available_bytes) }} FREE</b></div>
        <div class="disk-meter" :style="meterStyle(volumePercent(selected))"><i></i></div>
        <small>{{ formatBytes((selected.total_bytes ?? 0) - (selected.free_bytes ?? 0)) }} USED <span>{{ selected.total_bytes == null ? "—" : formatBytes(selected.total_bytes) }} TOTAL</span></small>
      </div>
    </aside>

    <section v-if="reviewScreen" class="review-workspace">
      <template v-if="removal">
        <header class="review-heading">
          <span class="metric-caption">SPACE · {{ removal.phase === "running" ? "REMOVAL IN PROGRESS" : "REMOVAL COMPLETE" }}</span>
          <h1>{{ removal.phase === "running" ? "Removing marked paths" : "Removal finished" }}</h1>
          <p v-if="removal.phase === 'running'">Core is removing each path after re-checking its guards. Closing the window is safe only after this finishes.</p>
          <p v-else>{{ removal.removed }} removed · {{ formatBytes(removal.bytes) }} · {{ removal.failed }} failed</p>
        </header>
        <div class="review-instrument">
          <div><span>{{ removal.phase === "running" ? "COMPLETED" : "REMOVED" }}</span><strong>{{ removal.phase === "running" ? `${removal.completed} / ${removal.total}` : removal.removed }}</strong></div>
          <div><span>FAILED</span><strong>{{ removal.phase === "running" ? removal.errors.length : removal.failed }}</strong></div>
          <div><span>FREE SPACE CHANGE</span><strong>{{ removalFreeDelta === null ? "MEASURING" : `${removalFreeDelta >= 0 ? "+" : "−"}${formatBytes(Math.abs(removalFreeDelta))}` }}</strong></div>
        </div>
        <div class="review-list">
          <div class="review-list-heading"><span>PROGRESS</span><span>{{ removal.total ? Math.round(removal.completed / removal.total * 100) : 0 }}%</span></div>
          <div class="removal-progress" role="progressbar" :aria-valuenow="removal.completed" aria-valuemin="0" :aria-valuemax="removal.total">
            <i :style="{ width: `${removal.total ? removal.completed / removal.total * 100 : 0}%` }"></i>
          </div>
          <div v-for="failure in removal.errors" :key="failure" class="review-blocked"><span>{{ failure }}</span></div>
        </div>
        <div class="review-actions">
          <button class="review-commit" :disabled="removal.phase === 'running'" @click="emit('finishRemoval')">RETURN TO MAP</button>
        </div>
      </template>
      <template v-else>
        <header class="review-heading">
          <span class="metric-caption">SPACE · DESTRUCTIVE ACTION REVIEW</span>
          <h1>Review marked paths</h1>
          <p>Nothing changes until final confirmation. Core will re-check every path and volume boundary.</p>
        </header>
        <div class="review-instrument">
          <div><span>TARGETS</span><strong>{{ Number(review?.targets ?? 0) }}</strong></div>
          <div><span>RECLAIM ON THIS VOLUME</span><strong>{{ formatBytes(Number(review?.reclaim_bytes ?? 0)) }}</strong></div>
          <div><span>OTHER VOLUME BYTES</span><strong>{{ formatBytes(Number(review?.foreign_bytes ?? 0)) }}</strong></div>
        </div>
        <div class="review-list">
          <div class="review-list-heading"><span>PATH / TARGET</span><span>ALLOCATED</span></div>
          <div v-for="mark in reviewMarks" :key="mark.path" class="review-path-row">
            <span><b>{{ mark.path }}</b><small>{{ mark.is_dir ? "DIRECTORY" : "FILE" }}</small></span>
            <strong>{{ formatBytes(mark.bytes) }}</strong>
            <button class="row-unmark" :aria-label="`Unmark ${mark.path}`" @click="emit('toggleMark', mark.path); emit('requestReview')">UNMARK</button>
          </div>
          <div v-for="blocked in blockedMarks" :key="blocked.path" class="review-blocked"><span>{{ blocked.path }}</span><b>BLOCKED · {{ blocked.reason }}</b></div>
          <p v-if="!reviewMarks.length" class="process-empty">NO MARKED PATHS TO REVIEW</p>
        </div>
        <div class="review-actions">
          <div class="removal-mode">
            <span>REMOVAL METHOD</span>
            <button :class="{ active: removalMode === 'trash' }" @click="emit('update:removalMode', 'trash')">SYSTEM TRASH</button>
            <button :class="{ active: removalMode === 'permanent' }" @click="emit('update:removalMode', 'permanent')">PERMANENT</button>
          </div>
          <button class="review-back" @click="emit('cancelReview')">RETURN TO MAP</button>
          <button class="review-commit" :disabled="Number(review?.targets ?? 0) === 0" @click="emit('update:removalConfirm', true)">CONTINUE TO CONFIRMATION ↗</button>
        </div>
        <div class="review-exports">
          <button :disabled="!reviewMarks.length" @click="emit('exportMarks', 'paths', true)">SAVE PATH LIST</button>
          <button :disabled="!reviewMarks.length" @click="emit('exportMarks', 'prompt', false)">COPY AGENT PROMPT</button>
          <button :disabled="!reviewMarks.length" @click="emit('clearMarks')">CLEAR MARKS</button>
          <span v-if="exportStatus" class="export-status">{{ exportStatus }}</span>
        </div>
      </template>
    </section>
    <p v-if="error" class="error" role="alert">{{ error }}</p>
    <div v-if="removalConfirm" class="removal-shade" role="dialog" aria-modal="true" aria-labelledby="removal-title">
      <section class="removal-confirm">
        <span class="metric-caption">DESTRUCTIVE OPERATION</span>
        <h2 id="removal-title">Confirm marked paths</h2>
        <p>{{ review?.targets }} targets · {{ formatBytes(Number(review?.reclaim_bytes ?? 0)) }} expected volume recovery</p>
        <label><input :checked="removalMode === 'trash'" type="radio" value="trash" @change="emit('update:removalMode', 'trash')"> Move to system Trash</label>
        <label><input :checked="removalMode === 'permanent'" type="radio" value="permanent" @change="emit('update:removalMode', 'permanent')"> Delete permanently</label>
        <div class="confirm-actions">
          <button @click="emit('cancelReview')">CANCEL</button>
          <button @click="emit('confirmRemoval', removalMode)">CONFIRM {{ removalMode === "trash" ? "TRASH" : "PERMANENT DELETE" }}</button>
        </div>
      </section>
    </div>
    <div v-if="helpOpen" class="removal-shade" role="dialog" aria-modal="true" aria-labelledby="help-title" @click.self="helpOpen = false">
      <section class="help-sheet">
        <header><span class="metric-caption">SPACE · KEYBOARD AND MOUSE</span><h2 id="help-title">Controls</h2></header>
        <dl>
          <template v-for="[keys, action] in helpRows" :key="keys">
            <dt>{{ keys }}</dt><dd>{{ action }}</dd>
          </template>
        </dl>
        <button class="review-back" @click="helpOpen = false">CLOSE · ESC</button>
      </section>
    </div>
    <aside v-if="hoveredTile?.tile.path && !reviewScreen && !pathMenu" class="tree-tooltip" :style="{ left: `${hoveredTile.x}px`, top: `${hoveredTile.y}px` }" aria-live="polite">
      <div class="tree-tooltip-title"><i>{{ hoveredTile.tile.is_dir ? "▰" : "▪" }}</i><b>{{ hoveredTile.tile.name }}</b></div>
      <small>{{ hoveredTile.tile.path }}</small>
      <div class="tree-tooltip-reading">
        <strong>{{ hoveredTile.tile.bytes === null ? "MERGED ITEMS" : formatBytes(hoveredTile.tile.bytes) }}</strong>
        <i :style="{ width: `${hoveredShare}%` }"></i><span>{{ hoveredShare.toFixed(1) }}%</span>
      </div>
      <span>{{ hoveredTile.tile.files?.toLocaleString() ?? "—" }} FILES · {{ hoveredTile.tile.dirs?.toLocaleString() ?? "—" }} DIRS · {{ formatBytes(hoveredTile.tile.direct_bytes ?? 0) }} DIRECT</span>
      <span v-if="hoveredTile.tile.name.startsWith('.')">HIDDEN</span>
      <span v-if="isHoveredMarked">MARKED FOR REMOVAL</span>
      <span v-else-if="isHoveredCovered">INSIDE MARKED {{ coveredMarkName }}</span>
      <span class="tree-tooltip-keys">{{ hoveredTile.tile.is_dir ? "SPACE MARK · ENTER OPEN" : "SPACE MARK" }}</span>
    </aside>
  </section>
  <Teleport to="body">
    <div v-if="pathMenu" ref="pathMenuEl" class="tree-context-menu" role="menu" :aria-label="t('space.pathActions')" :style="pathMenuStyle" @contextmenu.prevent @keydown="keyPathMenu">
      <button v-if="pathMenuCovered" role="menuitem" disabled>{{ t("space.insideMarked", { name: pathMenuCovered }) }}</button>
      <button v-else role="menuitem" @click="runPathMenu('toggleMark')">{{ pathMenuMarked ? t("space.unmark") : t("space.markForRemoval") }}</button>
      <button role="menuitem" @click="runPathMenu('revealPath')">{{ isMacOS ? t("space.revealFinder") : t("space.revealFileManager") }}</button>
    </div>
    <div v-if="volumeMenuOpen" class="volume-options" :class="{ above: volumeMenuAbove }" :style="volumeMenuStyle" role="listbox" aria-label="Select volume">
      <button v-for="volume in volumes" :key="volume.mount_point" role="option" :aria-selected="volume.mount_point === selectedVolume" :class="{ chosen: volume.mount_point === selectedVolume }" @click="onSelectVolume(volume.mount_point)">
        <i class="drive-lamp"></i>
        <span><b>{{ volume.mount_point }}</b><small>{{ volume.device || "LOCAL STORAGE" }}</small></span>
        <em>{{ volumePercent(volume).toFixed(0) }}% USED</em>
        <i class="option-check">{{ volume.mount_point === selectedVolume ? "✓" : "" }}</i>
      </button>
      <p v-if="!volumes.length">No mounted volumes</p>
    </div>
  </Teleport>
</template>
