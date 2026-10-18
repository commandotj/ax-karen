import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CliEvent } from "./cli";

const sent: Record<string, unknown>[] = [];
let emit: (event: CliEvent) => void = () => undefined;
const host = vi.hoisted(() => ({
  tauri: true,
  startError: undefined as Error | undefined,
  failCommand: "",
  windowCalls: [] as string[],
  trayStatus: [] as string[],
  emitTray: (_tab: string) => {},
}));

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => host.tauri,
  invoke: async (cmd: string, args?: { title?: string; summary?: string }) => {
    if (cmd === "update_tray_status" && args?.summary) {
      host.trayStatus.push(`${args.title ?? ""} | ${args.summary}`);
    }
  },
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (_event: string, handler: (event: { payload: string }) => void) => {
    host.emitTray = (payload: string) => handler({ payload });
    return () => {};
  },
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    startDragging: async () => { host.windowCalls.push("drag"); },
    minimize: async () => { host.windowCalls.push("minimize"); },
    toggleMaximize: async () => { host.windowCalls.push("maximize"); },
    close: async () => { host.windowCalls.push("close"); },
  }),
}));
vi.mock("./cli", () => ({
  KarenCli: class {
    constructor(onEvent: (event: CliEvent) => void) {
      emit = onEvent;
    }
    async start() {
      if (host.startError) throw host.startError;
    }
    async send(request: Record<string, unknown>) {
      if (request.command === host.failCommand) throw new Error(`${host.failCommand} refused`);
      sent.push(request);
    }
    async stop() {}
  },
}));

// isMacOS is read once at module load; the Full Disk Access hint is macOS-only.
Object.defineProperty(navigator, "userAgent", { value: "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)", configurable: true });

const GB = 1024 ** 3;
const volumes = [{ mount_point: "/", device: "/dev/disk1", total_bytes: 100 * GB, free_bytes: 40 * GB, available_bytes: 40 * GB }];
const crumbs = [{ path: "/", label: "/" }, { path: "/Users", label: "Users" }];
const tile = { name: "big.bin", path: "/Users/big.bin", category: "Other", bytes: GB, files: 1, is_dir: false, depth: 0, has_header: false, x: 0, y: 0, width: 200, height: 100 };

function treeView(overrides: Record<string, unknown> = {}): CliEvent {
  return {
    event: "tree_view", root: "/", path: "/Users", files: 1, dirs: 1, bytes: GB,
    selection: { name: "big.bin", path: "/Users/big.bin", category: "Other", bytes: GB, files: 1, is_dir: false, marked: false },
    siblings: [], marks: [], worth_a_look: [], breadcrumbs: crumbs, metric: "size", depth: 4, tiles: [tile],
    ...overrides,
  };
}

function lastRequest(command: string): Record<string, unknown> | undefined {
  return [...sent].reverse().find((request) => request.command === command);
}

let resize: (width: number, height: number) => void = () => undefined;

async function press(key: string, init: KeyboardEventInit = {}, target: EventTarget = window): Promise<KeyboardEvent> {
  const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init });
  target.dispatchEvent(event);
  await flushPromises();
  return event;
}

function countRequests(command: string): number {
  return sent.filter((request) => request.command === command).length;
}

async function button(wrapper: VueWrapper, text: string) {
  const found = wrapper.findAll("button").find((item) => item.text().includes(text));
  if (!found) throw new Error(`no button containing "${text}"`);
  return found;
}

describe("Karen desktop interface", () => {
  let wrapper: VueWrapper;

  beforeEach(async () => {
    sent.length = 0;
    // jsdom has no canvas; the treemap painter returns early without a 2d context.
    host.tauri = true;
    host.startError = undefined;
    host.failCommand = "";
    host.windowCalls.length = 0;
    HTMLCanvasElement.prototype.getContext = (() => null) as unknown as HTMLCanvasElement["getContext"];
    HTMLCanvasElement.prototype.getBoundingClientRect = () => ({ left: 0, top: 0, right: 1200, bottom: 680, width: 1200, height: 680, x: 0, y: 0, toJSON() {} });
    globalThis.ResizeObserver = class {
      constructor(callback: ResizeObserverCallback) { resize = (width, height) => callback([{ contentRect: { width, height } } as ResizeObserverEntry], this); }
      observe() {}
      unobserve() {}
      disconnect() {}
    };
    const { default: App } = await import("./App.vue");
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    emit({ event: "volumes", volumes });
    await flushPromises();
  });

  afterEach(() => wrapper.unmount());

  it("opens on Runtime and does not scan at launch", () => {
    expect(wrapper.find(".runtime-view").exists()).toBe(true);
    expect(lastRequest("scan")).toBeUndefined();
  });

  it("starts a scan on a cache miss, reports progress, then requests the cached view", async () => {
    await (await button(wrapper, "Space")).trigger("click");
    emit({ event: "tree_cache_miss", root: "/", path: "/" });
    await flushPromises();
    const scan = lastRequest("scan");
    expect(scan?.root).toBe("/");

    emit({ event: "scan_progress", id: scan?.id, files: 1200, dirs: 30, bytes: 5 * GB });
    await flushPromises();
    expect(wrapper.text()).toContain("MAPPING STORAGE");
    expect(wrapper.text()).toContain("1,200 files");

    emit({ event: "scan_complete", id: scan?.id, root: "/", files: 1, dirs: 1, bytes: GB, tiles: [tile] });
    await flushPromises();
    expect(lastRequest("tree_view")?.root).toBe("/");
  });

  it("shows scan errors and leaves the scan idle", async () => {
    await (await button(wrapper, "Space")).trigger("click");
    emit({ event: "tree_cache_miss", root: "/", path: "/" });
    await flushPromises();
    emit({ event: "error", id: lastRequest("scan")?.id, code: "scan_failed", message: "permission denied" });
    await flushPromises();
    expect(wrapper.text()).toContain("permission denied");
    expect(wrapper.text()).not.toContain("MAPPING…");
  });

  it("shows an explicit empty state for a scanned root with no entries", async () => {
    await (await button(wrapper, "Space")).trigger("click");
    emit(treeView({ tiles: [], files: 0, dirs: 0, bytes: 0 }));
    await flushPromises();
    expect(wrapper.text()).toContain("NO DIRECTORY ENTRIES");
  });

  it("navigates by breadcrumb using absolute paths and filters by name", async () => {
    await (await button(wrapper, "Space")).trigger("click");
    emit(treeView());
    await flushPromises();
    await wrapper.findAll(".tree-crumb").find((item) => item.text() === "Users")?.trigger("click");
    expect(lastRequest("tree_view")).toMatchObject({ root: "/", path: "/Users" });

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "/" }));
    await flushPromises();
    const input = wrapper.find<HTMLInputElement>(".tree-filter input");
    await input.setValue("big");
    expect(wrapper.text()).toContain("filter “big”");
    await input.trigger("keydown.esc");
    expect(wrapper.find<HTMLInputElement>(".tree-filter input").element.value).toBe("");
  });

  it("marks a path, opens Review, cancels it, and returns to the map", async () => {
    await (await button(wrapper, "Space")).trigger("click");
    emit(treeView());
    await flushPromises();
    await (await button(wrapper, "MARK FOR REMOVAL")).trigger("click");
    expect(lastRequest("toggle_mark")).toMatchObject({ root: "/", path: "/Users/big.bin" });

    emit({ event: "mark_changed", root: "/", path: "/Users/big.bin", marked: true, marks: [{ path: "/Users/big.bin", bytes: GB, is_dir: false, hidden: false }] });
    await flushPromises();
    await (await button(wrapper, "REVIEW 1 MARKS")).trigger("click");
    expect(lastRequest("review")?.root).toBe("/");

    emit({ event: "review", root: "/", marks: [{ path: "/Users/big.bin", bytes: GB, is_dir: false, hidden: false }], targets: 1, reclaim_bytes: GB, foreign_bytes: 0, blocked: [] });
    await flushPromises();
    expect(wrapper.find(".review-workspace").exists()).toBe(true);
    expect(wrapper.find(".tree-console").exists()).toBe(false);

    await (await button(wrapper, "RETURN TO MAP")).trigger("click");
    await flushPromises();
    expect(lastRequest("cancel_review")).toBeDefined();
    expect(wrapper.find(".review-workspace").exists()).toBe(false);
    expect(wrapper.find(".tree-console").exists()).toBe(true);
  });

  it("confirms removal, shows progress and the Done summary, then rescans on return", async () => {
    await (await button(wrapper, "Space")).trigger("click");
    emit(treeView());
    emit({ event: "review", root: "/", marks: [{ path: "/Users/big.bin", bytes: GB, is_dir: false, hidden: false }], targets: 2, reclaim_bytes: GB, foreign_bytes: 0, blocked: [] });
    await flushPromises();
    await (await button(wrapper, "CONTINUE TO CONFIRMATION")).trigger("click");
    expect(wrapper.find("[role=dialog]").exists()).toBe(true);
    await (await button(wrapper, "CONFIRM TRASH")).trigger("click");
    expect(lastRequest("confirm_remove")).toMatchObject({ mode: "trash" });

    emit({ event: "removal_started", mode: "trash" });
    emit({ event: "removal_progress", total: 2, completed: 0 });
    emit({ event: "removal_item", path: "/Users/a", ok: false, error: "busy" });
    await flushPromises();
    expect(wrapper.text()).toContain("Removing marked paths");
    expect(wrapper.text()).toContain("/Users/a · busy");

    emit({ event: "removal_complete", removed: 1, bytes: GB, failed: 1 });
    await flushPromises();
    expect(wrapper.text()).toContain("Removal finished");
    await (await button(wrapper, "RETURN TO MAP")).trigger("click");
    expect(lastRequest("scan")?.root).toBe("/");
  });

  it("cancels the running scan when another volume is chosen", async () => {
    await (await button(wrapper, "Space")).trigger("click");
    emit({ event: "tree_cache_miss", root: "/", path: "/" });
    await flushPromises();
    emit({ event: "volumes", volumes: [...volumes, { ...volumes[0], mount_point: "/Volumes/Data", device: "/dev/disk2" }] });
    await flushPromises();
    await (await button(wrapper, "Runtime")).trigger("click");
    await wrapper.findAll(".runtime-volume").find((item) => item.text().includes("/Volumes/Data"))?.trigger("click");
    expect(lastRequest("cancel_scan")).toBeDefined();
  });

  it("loads settings from the CLI at ready and forwards changes instead of using browser storage", async () => {
    emit({ event: "ready" });
    await flushPromises();
    expect(lastRequest("get_settings")).toBeDefined();

    emit({ event: "settings", power: "miser", ui_zoom: 1.2 });
    await flushPromises();
    expect(document.documentElement.style.getPropertyValue("zoom")).toBe("1.2");
    await (await button(wrapper, "Space")).trigger("click");
    emit(treeView());
    await flushPromises();
    expect(wrapper.find(".power-trigger").text()).toContain("MISER");

    await wrapper.find(".power-trigger").trigger("click");
    await (await button(wrapper, "Aggressive")).trigger("click");
    expect(lastRequest("set_settings")).toMatchObject({ power: "aggressive" });
  });
  async function openSpace(view: CliEvent = treeView()): Promise<void> {
    await (await button(wrapper, "Space")).trigger("click");
    emit(view);
    await flushPromises();
  }

  function canvas(): HTMLCanvasElement {
    return wrapper.find<HTMLCanvasElement>("canvas.treemap-canvas").element;
  }

  async function onCanvas(type: string, init: MouseEventInit = {}): Promise<void> {
    canvas().dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, ...init }));
    await flushPromises();
  }

  async function wheel(init: WheelEventInit): Promise<void> {
    canvas().dispatchEvent(new WheelEvent("wheel", { bubbles: true, cancelable: true, ...init }));
    await flushPromises();
  }

  // Four quadrant tiles on the default 1200x680 treemap.
  const quad = (name: string, x: number, y: number, extra: Record<string, unknown> = {}) => ({
    ...tile, name, path: `/Users/${name}`, x, y, width: 600, height: 340, ...extra,
  });
  const grid = [quad("a", 0, 0, { is_dir: true }), quad("b", 600, 0), quad("c", 0, 340), quad("d", 600, 340, { is_dir: true })];
  const gridView = (selected = "a", extra: Record<string, unknown> = {}) => treeView({
    tiles: grid,
    selection: { name: selected, path: `/Users/${selected}`, category: "Other", bytes: GB, is_dir: selected === "a", marked: false },
    ...extra,
  });

  function recordScale(): number[] {
    const scales: number[] = [];
    const context = new Proxy({}, {
      get: (_, key) => (...args: unknown[]) => {
        if (key === "scale") scales.push(Number(args[0]));
        if (key === "createLinearGradient") return { addColorStop() {} };
        if (key === "measureText") return { width: 0 };
        return undefined;
      },
      set: () => true,
    });
    HTMLCanvasElement.prototype.getContext = (() => context) as unknown as HTMLCanvasElement["getContext"];
    return scales;
  }

  describe("host and telemetry", () => {
    it("reports when it is not running inside Karen.app", async () => {
      wrapper.unmount();
      host.tauri = false;
      wrapper = mount((await import("./App.vue")).default, { attachTo: document.body });
      await flushPromises();
      expect(wrapper.text()).toContain("Open Karen.app to connect the CLI");
    });

    it("shows a fault when the CLI sidecar cannot start", async () => {
      wrapper.unmount();
      host.startError = new Error("sidecar missing");
      wrapper = mount((await import("./App.vue")).default, { attachTo: document.body });
      await flushPromises();
      expect(wrapper.text()).toContain("CLI unavailable");
      await (await button(wrapper, "Space")).trigger("click");
      expect(wrapper.find("[role=alert]").text()).toBe("Error: sidecar missing");
      expect(wrapper.find(".telemetry-state.fault").exists()).toBe(true);
    });

    it("requests resources and volumes at start and forwards window controls", async () => {
      expect(countRequests("resources")).toBe(1);
      expect(countRequests("volumes")).toBe(1);
      await wrapper.find("[aria-label='Minimize window']").trigger("click");
      await wrapper.find("[aria-label='Toggle window size']").trigger("click");
      await wrapper.find("[aria-label='Close window']").trigger("click");
      await wrapper.find(".masthead").trigger("mousedown");
      expect(host.windowCalls).toEqual(["minimize", "maximize", "close", "drag"]);
    });

    it("renders CPU and memory readings, charts and load conditions on Runtime", async () => {
      expect(wrapper.text()).toContain("INITIALIZING");
      expect(wrapper.text()).toContain("SAMPLING BASELINE");
      expect(wrapper.text()).toContain("PROCESS SAMPLE UNAVAILABLE");
      const resources = (cpu: number) => emit({
        event: "resources", cpu_percent: cpu, memory_used_bytes: 6 * GB, memory_total_bytes: 8 * GB,
        cpu_processes_available: true, cpu_processes: [{ pid: 7, name: "cargo", cpu_percent: 90.5 }],
        memory_processes_available: true, memory_processes: [{ pid: 7, name: "cargo", memory_bytes: GB }],
      });
      resources(20);
      await flushPromises();
      expect(wrapper.text()).toContain("NOMINAL");
      expect(wrapper.find(".cpu-line").attributes("d")).toBe("");
      resources(60);
      await flushPromises();
      expect(wrapper.text()).toContain("ACTIVE");
      expect(wrapper.find(".cpu-line").attributes("d")).toBe("M0.0,80.0 L1000.0,40.0");
      expect(wrapper.find(".chart-area").exists()).toBe(true);
      resources(90);
      await flushPromises();
      expect(wrapper.text()).toContain("HIGH LOAD");
      expect(wrapper.text()).toContain("PID 7");
      expect(wrapper.text()).toContain("6.0 GB");
      expect(wrapper.text()).toContain("75%");
      expect(wrapper.text()).toContain("2.0 GB");
      // The history window keeps the most recent 90 samples.
      for (let index = 0; index < 90; index += 1) resources(10);
      await flushPromises();
      expect(wrapper.find(".cpu-line").attributes("d")?.split(" ")).toHaveLength(90);
    });

    it("drops malformed process readings and keeps at most 20", async () => {
      const cpuRows = [null, 3, { pid: 1.5, name: "x", cpu_percent: 1 }, { pid: 2, name: 4, cpu_percent: 1 },
        { pid: 3, name: "neg", cpu_percent: -1 }, { pid: 4, name: "inf", cpu_percent: Infinity },
        ...Array.from({ length: 25 }, (_, index) => ({ pid: 100 + index, name: `p${index}`, cpu_percent: 0 }))];
      const memoryRows = [null, { pid: 1.5, name: "x", memory_bytes: 1 }, { pid: 2, name: "x", memory_bytes: "1" },
        { pid: 3, name: "neg", memory_bytes: -1 }, { pid: 4, name: "frac", memory_bytes: 1.5 },
        ...Array.from({ length: 25 }, (_, index) => ({ pid: 200 + index, name: `m${index}`, memory_bytes: 0 }))];
      emit({ event: "resources", cpu_percent: "?", cpu_processes_available: true, cpu_processes: cpuRows, memory_processes_available: true, memory_processes: memoryRows });
      await flushPromises();
      await (await button(wrapper, "CPU")).trigger("click");
      expect(wrapper.text()).toContain("SYSTEM —%");
      const cpuRowsShown = wrapper.findAll(".detail-process-row");
      expect(cpuRowsShown).toHaveLength(20);
      expect(cpuRowsShown[0].text()).toContain("p0");
      // All loads are zero, so the bars are empty instead of dividing by zero.
      expect(cpuRowsShown[0].find(".detail-process-reading i").attributes("style")).toContain("width: 0%");
      await (await button(wrapper, "Memory")).trigger("click");
      expect(wrapper.findAll(".detail-process-row")).toHaveLength(20);
      expect(wrapper.find(".detail-process-reading i").attributes("style")).toContain("width: 0%");
      emit({ event: "resources", cpu_processes: "bad", memory_processes: "bad" });
      await flushPromises();
      expect(wrapper.text()).toContain("PROCESS MEMORY SAMPLE UNAVAILABLE");
      expect(wrapper.text()).toContain("INITIALIZING");
    });

    it("shows empty CPU and memory process states and proportional bars", async () => {
      emit({ event: "resources", cpu_percent: 5, memory_used_bytes: 1, memory_total_bytes: 4, cpu_processes_available: true, cpu_processes: [], memory_processes_available: true, memory_processes: [] });
      await flushPromises();
      expect(wrapper.text()).toContain("NO PROCESS CPU READINGS AVAILABLE");
      await (await button(wrapper, "CPU")).trigger("click");
      expect(wrapper.text()).toContain("NO PROCESS CPU READINGS AVAILABLE");
      await (await button(wrapper, "Memory")).trigger("click");
      expect(wrapper.text()).toContain("NO PROCESS MEMORY READINGS AVAILABLE");
      expect(wrapper.text()).toContain("25% COMMITTED");
      emit({
        event: "resources", cpu_percent: 5, cpu_processes_available: true, memory_processes_available: true,
        cpu_processes: [{ pid: 1, name: "a", cpu_percent: 50 }, { pid: 2, name: "b", cpu_percent: 25 }],
        memory_processes: [{ pid: 1, name: "a", memory_bytes: 400 }, { pid: 2, name: "b", memory_bytes: 100 }],
      });
      await flushPromises();
      expect(wrapper.findAll(".detail-process-reading i").map((bar) => bar.attributes("style"))).toEqual(["width: 100%;", "width: 25%;"]);
      await (await button(wrapper, "CPU")).trigger("click");
      expect(wrapper.findAll(".detail-process-reading i").map((bar) => bar.attributes("style"))).toEqual(["width: 100%;", "width: 50%;"]);
      await (await button(wrapper, "Runtime")).trigger("click");
      await (await button(wrapper, "MEMORY DETAIL")).trigger("click");
      expect(wrapper.find(".memory-view").exists()).toBe(true);
    });

    it("shows a waiting state before any volume is reported", async () => {
      emit({ event: "volumes", volumes: [] });
      await flushPromises();
      expect(wrapper.text()).toContain("WAITING FOR MOUNTED VOLUME TELEMETRY");
    });

    it("reports protocol errors outside a scan with the code as a fallback", async () => {
      emit({ event: "error", code: "bad_request" });
      await flushPromises();
      expect(wrapper.text()).toContain("Monitor fault");
      await (await button(wrapper, "Space")).trigger("click");
      expect(wrapper.find("[role=alert]").text()).toBe("bad_request");
      emit({ event: "error" });
      await flushPromises();
      expect(wrapper.find("[role=alert]").text()).toBe("Unknown error");
      emit({ event: "error", id: "reveal-1", message: "reveal failed" });
      await flushPromises();
      expect(wrapper.find("[role=alert]").text()).toBe("reveal failed");
    });
  });

  describe("scanning", () => {
    it("ignores events from other scans and applies its own", async () => {
      await (await button(wrapper, "Space")).trigger("click");
      emit({ event: "tree_cache_miss", root: "/" });
      await flushPromises();
      const id = lastRequest("scan")?.id;
      emit({ event: "scan_started", id: "other", files: 99 });
      emit({ event: "scan_preview", id: "other", files: 99 });
      emit({ event: "scan_complete", id: "other" });
      emit({ event: "error", id: "other", message: "ignored" });
      await flushPromises();
      expect(wrapper.text()).toContain("0 files");
      expect(wrapper.text()).not.toContain("ignored");

      emit({ event: "scan_started", id, workers: 6 });
      emit({ event: "scan_preview", id, files: 300, bytes: 2048 });
      await flushPromises();
      expect(wrapper.find(".power-trigger").text()).toContain("6 WORKERS");
      expect(wrapper.text()).toContain("300 files · 2.0 KB inspected · previous completed map retained");
      expect(wrapper.text()).toContain("SCANNING · PREVIOUS MAP RETAINED");
      emit({ event: "scan_preview", id });
      emit({ event: "scan_progress", id });
      await flushPromises();
      expect(wrapper.text()).toContain("300 files");

      emit({ event: "scan_complete", id, root: "/", unreadable: 4 });
      await flushPromises();
      expect(lastRequest("tree_view")).toMatchObject({ root: "/", path: "/" });
      expect(wrapper.text()).toContain("NO DIRECTORY ENTRIES");
      expect(wrapper.text()).toContain("4 unreadable");
      expect(wrapper.text()).toContain("4 entries could not be read");
      await (await button(wrapper, "OPEN PRIVACY SETTINGS")).trigger("click");
      expect(lastRequest("open_full_disk_access")).toBeDefined();
    });

    it("shows an error when the scan request cannot be sent", async () => {
      host.failCommand = "scan";
      await (await button(wrapper, "Space")).trigger("click");
      emit({ event: "tree_cache_miss", path: "/" });
      await flushPromises();
      expect(wrapper.find("[role=alert]").text()).toContain("scan refused");
      expect(wrapper.find(".map-button").text()).toBe("MAP VOLUME");
    });

    it("finishes a pending volume switch after the cancelled scan reports its error", async () => {
      emit({ event: "volumes", volumes: [...volumes, { mount_point: "/Volumes/Data" }] });
      await (await button(wrapper, "Space")).trigger("click");
      emit({ event: "tree_cache_miss", root: "/" });
      await flushPromises();
      await wrapper.find(".volume-picker-trigger").trigger("click");
      const option = [...document.querySelectorAll<HTMLButtonElement>(".volume-options button")].find((item) => item.textContent?.includes("/Volumes/Data"));
      option?.click();
      await flushPromises();
      expect(lastRequest("cancel_scan")).toBeDefined();
      emit({ event: "error", id: lastRequest("scan")?.id, code: "cancelled" });
      await flushPromises();
      expect(lastRequest("tree_view")).toMatchObject({ root: "/Volumes/Data", path: "/Volumes/Data" });
      expect(wrapper.find("[role=alert]").exists()).toBe(false);
      expect(wrapper.text()).toContain("LOCAL STORAGE");
    });

    it("finishes a pending volume switch when the scan completes first", async () => {
      emit({ event: "volumes", volumes: [...volumes, { mount_point: "/Volumes/Data", total_bytes: 0 }] });
      await (await button(wrapper, "Space")).trigger("click");
      emit({ event: "tree_cache_miss", root: "/" });
      await flushPromises();
      const id = lastRequest("scan")?.id;
      await (await button(wrapper, "Runtime")).trigger("click");
      await wrapper.findAll(".runtime-volume")[1].trigger("click");
      emit({ event: "scan_complete", id, root: "/", tiles: "none" });
      await flushPromises();
      expect(lastRequest("tree_view")).toMatchObject({ root: "/Volumes/Data", path: "/Volumes/Data" });
    });

    it("rescans with hidden files and apparent size toggles and from the map button", async () => {
      await openSpace();
      const [hidden, apparent] = wrapper.findAll<HTMLInputElement>(".tree-check input");
      await hidden.setValue(false);
      expect(lastRequest("scan")).toMatchObject({ root: "/", include_hidden: false, apparent_size: false, power: "balanced" });
      emit({ event: "scan_complete", id: lastRequest("scan")?.id, root: "/" });
      await flushPromises();
      await apparent.setValue(true);
      expect(lastRequest("scan")).toMatchObject({ apparent_size: true });
      emit({ event: "scan_complete", id: lastRequest("scan")?.id, root: "/" });
      await flushPromises();
      await (await button(wrapper, "RESCAN")).trigger("click");
      expect(countRequests("scan")).toBe(3);
      expect(wrapper.find(".map-button").text()).toBe("MAPPING…");
    });

    it("scans a directory chosen in the native dialog", async () => {
      await openSpace();
      await press("o", { metaKey: true });
      expect(lastRequest("choose_directory")).toBeDefined();
      emit({ event: "directory_chosen", path: "/Users/me/Projects" });
      emit({ event: "directory_chosen" });
      await flushPromises();
      expect(lastRequest("scan")?.root).toBe("/Users/me/Projects");
      expect(countRequests("scan")).toBe(1);
    });
  });

  describe("volume picker", () => {
    it("opens below the trigger, picks the current volume and closes on outside pointer or Escape", async () => {
      await (await button(wrapper, "Space")).trigger("click");
      await wrapper.find(".volume-picker-trigger").trigger("click");
      const menu = document.querySelector<HTMLElement>(".volume-options");
      expect(menu?.classList.contains("above")).toBe(false);
      expect(menu?.style.top).toBe("6px");
      expect(menu?.style.left).toBe("16px");
      expect(menu?.style.width).toBe("410px");
      expect(menu?.style.maxHeight).toBe("58px");
      expect(menu?.textContent).toContain("✓");
      window.dispatchEvent(new Event("resize"));
      window.dispatchEvent(new Event("scroll"));
      menu?.dispatchEvent(new Event("pointerdown", { bubbles: true }));
      await flushPromises();
      expect(document.querySelector(".volume-options")).not.toBeNull();
      document.body.dispatchEvent(new Event("pointerdown", { bubbles: true }));
      await flushPromises();
      expect(document.querySelector(".volume-options")).toBeNull();

      await wrapper.find(".volume-picker-trigger").trigger("click");
      await press("Escape");
      expect(document.querySelector(".volume-options")).toBeNull();

      await wrapper.find(".volume-picker-trigger").trigger("click");
      document.querySelector<HTMLButtonElement>(".volume-options button")?.click();
      await flushPromises();
      expect(lastRequest("tree_view")).toMatchObject({ root: "/", path: "/", open: true });
      await wrapper.find(".volume-picker-trigger").trigger("click");
      await wrapper.find(".volume-picker-trigger").trigger("click");
      expect(document.querySelector(".volume-options")).toBeNull();
    });

    it("opens above the trigger near the bottom of the window", async () => {
      await (await button(wrapper, "Space")).trigger("click");
      const trigger = wrapper.find(".volume-picker-trigger").element;
      trigger.getBoundingClientRect = () => ({ left: 900, top: 700, right: 1000, bottom: 730, width: 100, height: 30, x: 900, y: 700, toJSON() {} });
      await wrapper.find(".volume-picker-trigger").trigger("click");
      const menu = document.querySelector<HTMLElement>(".volume-options");
      expect(menu?.classList.contains("above")).toBe(true);
      expect(menu?.style.top).toBe("636px");
      expect(menu?.style.left).toBe("590px");
    });

    it("opens another volume directly when no scan is running", async () => {
      emit({ event: "volumes", volumes: [...volumes, { mount_point: "/Volumes/Data" }] });
      await flushPromises();
      await wrapper.findAll(".runtime-volume")[1].trigger("click");
      expect(lastRequest("cancel_scan")).toBeUndefined();
      expect(lastRequest("tree_view")).toMatchObject({ root: "/Volumes/Data", path: "/Volumes/Data", open: true });
      expect(wrapper.find(".volume-picker-trigger").text()).toContain("/Volumes/Data");
    });

    it("lists no volumes and does not reopen the current mapped volume", async () => {
      await openSpace(treeView({ tiles: [tile] }));
      const before = countRequests("tree_view");
      await (await button(wrapper, "Runtime")).trigger("click");
      await wrapper.find(".runtime-volume").trigger("click");
      expect(countRequests("tree_view")).toBe(before);
      emit({ event: "volumes", volumes: [] });
      await flushPromises();
      await wrapper.find(".volume-picker-trigger").trigger("click");
      expect(document.querySelector(".volume-options")?.textContent).toContain("No mounted volumes");
    });
  });

  describe("disk keyboard", () => {
    it("moves selection with arrows and hjkl toward the nearest tile", async () => {
      await openSpace(gridView());
      const moved = async (key: string) => {
        await press(key);
        return lastRequest("tree_view");
      };
      expect(await moved("ArrowRight")).toMatchObject({ path: "/Users/b", open: false });
      expect(await moved("j")).toMatchObject({ path: "/Users/c" });
      emit(gridView("d"));
      await flushPromises();
      expect(await moved("k")).toMatchObject({ path: "/Users/b" });
      expect(await moved("h")).toMatchObject({ path: "/Users/c" });
      // Without a selected tile, movement starts from the first tile.
      emit(gridView("missing"));
      await flushPromises();
      expect(await moved("ArrowRight")).toMatchObject({ path: "/Users/b" });
    });

    it("ignores arrows with nothing to select and keys from inputs, buttons, modifiers and other views", async () => {
      await openSpace(treeView({ tiles: [], selection: null }));
      await press("ArrowLeft");
      const count = sent.length;
      await press("x", {}, wrapper.find(".map-button").element);
      await press("g", { altKey: true });
      emit(treeView());
      await flushPromises();
      await press("x", {}, wrapper.find(".tree-filter input").element);
      await (await button(wrapper, "Runtime")).trigger("click");
      await press("g");
      window.dispatchEvent(new MouseEvent("mouseup", { button: 3 }));
      expect(sent.length).toBe(count);
    });

    it("acts on the pointed tile after the pointer moved, and on the selection after a keyboard move", async () => {
      await openSpace(gridView("c"));
      // Selection is c; the pointer rests over d (a directory), so Enter and X act on d.
      await onCanvas("pointermove", { clientX: 900, clientY: 500 });
      await press("Enter");
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/d", open: true });
      await press("x");
      expect(lastRequest("toggle_mark")).toMatchObject({ path: "/Users/d" });
      // An arrow key hands the target back to the keyboard selection.
      await press("ArrowRight");
      await press("x");
      expect(lastRequest("toggle_mark")).toMatchObject({ path: "/Users/c" });
    });

    it("keeps the selection when no tile lies in the pressed direction", async () => {
      await openSpace(gridView("b"));
      const before = countRequests("tree_view");
      const event = await press("ArrowRight");
      expect(countRequests("tree_view")).toBe(before);
      expect(event.defaultPrevented).toBe(false);
    });

    it("opens directories with Enter and marks with Space or X", async () => {
      await openSpace(gridView("a"));
      const enter = await press("Enter");
      expect(enter.defaultPrevented).toBe(true);
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/a", open: true });
      await press(" ", { code: "Space" });
      expect(lastRequest("toggle_mark")).toMatchObject({ path: "/Users/a" });
      emit(gridView("b"));
      await flushPromises();
      const count = countRequests("tree_view");
      await press("Enter");
      expect(countRequests("tree_view")).toBe(count);
      await press("x");
      expect(lastRequest("toggle_mark")).toMatchObject({ path: "/Users/b" });
      emit(treeView({ selection: null }));
      await flushPromises();
      await press("x");
      await press("Enter");
      await press("o");
      expect(countRequests("toggle_mark")).toBe(2);
      expect(lastRequest("reveal")).toBeUndefined();
    });

    it("handles help, widen, reveal, review, quit and panel keys", async () => {
      await openSpace(treeView({ marks: [{ path: "/Users/big.bin", bytes: GB, is_dir: false, hidden: false }] }));
      await press("?");
      expect(wrapper.find("#help-title").exists()).toBe(true);
      const count = sent.length;
      await press("g");
      expect(sent.length).toBe(count);
      await press("Escape");
      expect(wrapper.find("#help-title").exists()).toBe(false);
      await press("?");
      await (await button(wrapper, "CLOSE · ESC")).trigger("click");
      expect(wrapper.find("#help-title").exists()).toBe(false);
      await press("?");
      await wrapper.find(".removal-shade").trigger("click");
      expect(wrapper.find("#help-title").exists()).toBe(false);

      await press("g");
      expect(lastRequest("tree_view")).toMatchObject({ root: "/", path: "/", open: true });
      await press("o");
      expect(lastRequest("reveal")).toMatchObject({ path: "/Users/big.bin" });
      emit({ event: "reveal_done" });
      await press("q");
      expect(host.windowCalls).toContain("close");
      await press("p");
      expect(wrapper.find(".tree-inspector").exists()).toBe(false);
      await press("p");
      expect(wrapper.find(".tree-inspector").exists()).toBe(true);
      await press("c");
      expect(lastRequest("review")?.root).toBe("/");
    });

    it("goes up with u, Backspace and Escape, and Escape clears the filter first", async () => {
      await openSpace();
      await press("u");
      expect(lastRequest("tree_view")).toMatchObject({ path: "/", open: true });
      sent.length = 0;
      await press("Backspace");
      expect(lastRequest("tree_view")).toMatchObject({ path: "/" });
      await press("s");
      await wrapper.find(".tree-filter input").setValue("big");
      sent.length = 0;
      await press("Escape");
      expect(wrapper.find<HTMLInputElement>(".tree-filter input").element.value).toBe("");
      expect(lastRequest("tree_view")).toBeUndefined();
      await press("Escape");
      expect(lastRequest("tree_view")).toMatchObject({ path: "/" });
      emit(treeView({ breadcrumbs: [{ path: "/", label: "/" }] }));
      await flushPromises();
      sent.length = 0;
      await press("u");
      expect(lastRequest("tree_view")).toBeUndefined();
    });

    it("steps through siblings with Tab and Shift-Tab", async () => {
      const siblings = ["x", "y", "z"].map((name) => ({ name, path: `/Users/${name}`, bytes: GB, is_dir: false }));
      await openSpace(treeView({ siblings, selection: { name: "y", path: "/Users/y", bytes: GB } }));
      await press("Tab");
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/z", open: false });
      await press("Tab", { shiftKey: true });
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/x" });
      emit(treeView({ siblings: [] }));
      await flushPromises();
      sent.length = 0;
      await press("Tab");
      expect(sent).toEqual([]);
    });

    it("toggles size modes, depth and rescans from the keyboard", async () => {
      await openSpace();
      await press("t");
      expect(lastRequest("tree_view")).toMatchObject({ metric: "files", path: "/Users" });
      await press("T");
      expect(wrapper.find(".tree-legend.age-mode").exists()).toBe(true);
      expect(lastRequest("tree_view")).toMatchObject({ metric: "size" });
      await press("t");
      expect(wrapper.find(".tree-mode .active").text()).toBe("SIZE");
      await press("[");
      expect(lastRequest("tree_view")).toMatchObject({ depth: 3 });
      await press("]");
      await press("]");
      await press("]");
      await press("]");
      expect(lastRequest("tree_view")).toMatchObject({ depth: 6 });
      expect(wrapper.find("[aria-label='Increase treemap depth']").attributes("disabled")).toBeDefined();
      await wrapper.find("[aria-label='Decrease treemap depth']").trigger("click");
      expect(wrapper.text()).toContain("DEPTH 5");
      await (await button(wrapper, "FILES")).trigger("click");
      expect(lastRequest("tree_view")).toMatchObject({ metric: "files" });

      await press("d");
      expect(lastRequest("scan")).toMatchObject({ apparent_size: true });
      const scans = countRequests("scan");
      await press("i");
      await press("d");
      await press("r");
      expect(countRequests("scan")).toBe(scans);
      emit({ event: "scan_complete", id: lastRequest("scan")?.id, root: "/" });
      await flushPromises();
      await press("i");
      expect(lastRequest("scan")).toMatchObject({ include_hidden: false });
      emit({ event: "scan_complete", id: lastRequest("scan")?.id, root: "/" });
      await flushPromises();
      await press("R");
      expect(countRequests("scan")).toBe(scans + 2);
    });

    it("does not request a view for mode or depth changes without a directory", async () => {
      await (await button(wrapper, "Space")).trigger("click");
      sent.length = 0;
      await press("t");
      await press("]");
      expect(sent).toEqual([]);
    });

    it("zooms the treemap view with - = + and 0", async () => {
      await openSpace(gridView());
      const scales = recordScale();
      await press("=");
      expect(scales.at(-1)).toBe(1.25);
      await press("+");
      expect(scales.at(-1)).toBeCloseTo(1.5625);
      await press("-");
      expect(scales.at(-1)).toBeCloseTo(1.25);
      await press("0");
      expect(scales.at(-1)).toBe(1);
      await press("-");
      expect(scales.at(-1)).toBe(1);
    });
  });

  describe("global keys and history", () => {
    it("changes interface zoom with Cmd/Ctrl + - 0 on any view", async () => {
      await press("=", { metaKey: true });
      expect(lastRequest("set_settings")).toMatchObject({ ui_zoom: 1.1 });
      await press("+", { ctrlKey: true });
      expect(document.documentElement.style.getPropertyValue("zoom")).toBe("1.2");
      await press("-", { metaKey: true });
      expect(lastRequest("set_settings")).toMatchObject({ ui_zoom: 1.1 });
      await press("0", { metaKey: true });
      expect(lastRequest("set_settings")).toMatchObject({ ui_zoom: 1 });
      emit({ event: "settings", ui_zoom: 5, power: "turbo" });
      await flushPromises();
      expect(document.documentElement.style.getPropertyValue("zoom")).toBe("2");
      emit({ event: "settings" });
      const count = sent.length;
      await press("o", { metaKey: true });
      expect(sent.length).toBe(count);
    });

    it("walks history with buttons, Alt-arrows, Cmd-[ ] and mouse side buttons", async () => {
      await openSpace();
      const deeper = treeView({ breadcrumbs: [...crumbs, { path: "/Users/a", label: "a" }] });
      emit(deeper);
      await flushPromises();
      expect(wrapper.find("[aria-label='History back']").attributes("title")).toBe("/Users");
      expect(wrapper.find("[aria-label='History forward']").attributes("title")).toBe("Nothing later");

      await press("ArrowLeft", { altKey: true });
      expect(lastRequest("tree_view")).toMatchObject({ root: "/", path: "/Users", open: true });
      emit(treeView());
      await flushPromises();
      expect(wrapper.find("[aria-label='History forward']").attributes("title")).toBe("/Users/a");
      sent.length = 0;
      await press("[", { metaKey: true });
      expect(sent).toEqual([]);

      await press("]", { metaKey: true });
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/a" });
      emit(deeper);
      await press("ArrowRight", { altKey: true });
      window.dispatchEvent(new MouseEvent("mouseup", { button: 3, cancelable: true }));
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users" });
      emit(treeView());
      window.dispatchEvent(new MouseEvent("mouseup", { button: 4, cancelable: true }));
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/a" });
      emit(deeper);
      await flushPromises();
      window.dispatchEvent(new MouseEvent("mouseup", { button: 0 }));
      await wrapper.find("[aria-label='History back']").trigger("click");
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users" });
      emit(treeView());
      await flushPromises();
      await wrapper.find("[aria-label='History forward']").trigger("click");
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/a" });
      // A new view after going back drops the forward entries.
      emit(deeper);
      await press("ArrowLeft", { altKey: true });
      emit(treeView());
      emit(treeView({ breadcrumbs: [...crumbs, { path: "/Users/b", label: "b" }] }));
      await flushPromises();
      expect(wrapper.find("[aria-label='History back']").attributes("title")).toBe("/Users");
      expect(wrapper.find("[aria-label='History forward']").attributes("disabled")).toBeDefined();
    });
  });

  describe("treemap pointer", () => {
    it("selects on click, opens a selected or double-clicked directory, and ignores misses", async () => {
      await openSpace(gridView("b"));
      await onCanvas("click", { clientX: 100, clientY: 100, detail: 1 });
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/a", open: false });
      await onCanvas("click", { clientX: 100, clientY: 100, detail: 2 });
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/a", open: true });
      emit(gridView("a"));
      await flushPromises();
      await onCanvas("click", { clientX: 100, clientY: 100, detail: 1 });
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/a", open: true });
      await onCanvas("click", { clientX: 700, clientY: 100, detail: 2 });
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/b", open: false });
      sent.length = 0;
      await onCanvas("click", { clientX: 1300, clientY: 100 });
      await onCanvas("contextmenu", { clientX: 1300, clientY: 100 });
      emit(treeView({ tiles: [{ ...tile, path: null }] }));
      await flushPromises();
      await onCanvas("click", { clientX: 10, clientY: 10 });
      expect(sent).toEqual([]);
    });

    it("marks with Ctrl/Cmd-click and middle-click and reveals on right-click", async () => {
      await openSpace(gridView());
      await onCanvas("click", { clientX: 700, clientY: 100, ctrlKey: true });
      expect(lastRequest("toggle_mark")).toMatchObject({ path: "/Users/b" });
      await onCanvas("click", { clientX: 100, clientY: 400, metaKey: true });
      expect(lastRequest("toggle_mark")).toMatchObject({ path: "/Users/c" });
      await onCanvas("auxclick", { clientX: 700, clientY: 400, button: 1 });
      expect(lastRequest("toggle_mark")).toMatchObject({ path: "/Users/d" });
      await onCanvas("contextmenu", { clientX: 700, clientY: 400 });
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/d", open: false });
      expect(lastRequest("reveal")).toMatchObject({ path: "/Users/d" });
    });

    it("shows a tooltip that flips near window edges and describes the tile", async () => {
      const marks = [{ path: "/Users/a", bytes: GB, is_dir: true, hidden: false }, { path: "/Users/d", bytes: GB, is_dir: true, hidden: false }];
      const tiles = [
        quad("a", 0, 0, { is_dir: true, parent_bytes: 4 * GB, dirs: 2, files: 1000, direct_bytes: 2048 }),
        quad(".b", 600, 0, { bytes: null, files: null, dirs: null }),
        quad("c", 0, 340, { bytes: 2 * GB, parent_bytes: GB }),
        quad("d/inner", 600, 340, { is_dir: true }),
      ];
      await openSpace(treeView({ tiles, marks }));
      await onCanvas("pointermove", { clientX: 100, clientY: 100 });
      let tip = wrapper.find(".tree-tooltip");
      expect(tip.attributes("style")).toContain("left: 112px; top: 112px");
      expect(tip.text()).toContain("25.0%");
      expect(tip.text()).toContain("1,000 FILES · 2 DIRS · 2.0 KB DIRECT");
      expect(tip.text()).toContain("MARKED FOR REMOVAL");
      expect(tip.text()).toContain("SPACE MARK · ENTER OPEN");
      await onCanvas("pointermove", { clientX: 900, clientY: 670 });
      tip = wrapper.find(".tree-tooltip");
      expect(tip.attributes("style")).toContain("left: 628px; top: 516px");
      expect(tip.text()).toContain("INSIDE MARKED d");
      await onCanvas("pointermove", { clientX: 1020, clientY: 100 });
      tip = wrapper.find(".tree-tooltip");
      expect(tip.text()).toContain("MERGED ITEMS");
      expect(tip.text()).toContain("100.0%");
      expect(tip.text()).toContain("— FILES · — DIRS · 0 B DIRECT");
      expect(tip.text()).toContain("HIDDEN");
      expect(tip.text()).not.toContain("ENTER OPEN");
      await onCanvas("pointermove", { clientX: 100, clientY: 400 });
      expect(wrapper.find(".tree-tooltip").text()).toContain("100.0%");
      await onCanvas("pointermove", { clientX: 1300, clientY: 400 });
      expect(wrapper.find(".tree-tooltip").exists()).toBe(false);
      await onCanvas("pointermove", { clientX: 100, clientY: 100 });
      await onCanvas("pointerleave");
      expect(wrapper.find(".tree-tooltip").exists()).toBe(false);
      await onCanvas("pointermove", { clientX: 100, clientY: 100 });
      await (await button(wrapper, "Runtime")).trigger("click");
      await (await button(wrapper, "Space")).trigger("click");
      expect(wrapper.find(".tree-tooltip").exists()).toBe(false);
    });

    it("zooms toward the pointer, pans with shift, descends into directories and ascends at full size", async () => {
      await openSpace(gridView());
      const scales = recordScale();
      await wheel({ clientX: 300, clientY: 170, deltaY: -200 });
      expect(scales.at(-1)).toBeCloseTo(Math.exp(0.3));
      await wheel({ clientX: 300, clientY: 170, deltaY: 0, shiftKey: true, deltaX: 50 });
      await wheel({ clientX: 300, clientY: 170, deltaY: 50, shiftKey: true });
      sent.length = 0;
      // Deep zoom over a file only magnifies.
      await wheel({ clientX: 1100, clientY: 170, deltaY: -2000 });
      expect(scales.at(-1)).toBe(8);
      expect(lastRequest("tree_view")).toBeUndefined();
      await press("0");
      await wheel({ clientX: 300, clientY: 170, deltaY: -1000 });
      expect(scales.at(-1)).toBeGreaterThan(3);
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/a", open: true });
      emit(gridView("a", { breadcrumbs: [...crumbs, { path: "/Users/a", label: "a" }] }));
      await flushPromises();
      expect(scales.at(-1)).toBe(1);
      await wheel({ clientX: 300, clientY: 170, deltaY: 100 });
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users", open: true });
      emit(gridView("a", { breadcrumbs: [{ path: "/", label: "/" }] }));
      await flushPromises();
      sent.length = 0;
      await wheel({ clientX: 300, clientY: 170, deltaY: 100 });
      expect(sent).toEqual([]);
    });

    it("requests a relaid view when the treemap is resized", async () => {
      await openSpace();
      resize(900, 500);
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users", width: 900, height: 500 });
      sent.length = 0;
      resize(900.2, 500);
      resize(100, 100);
      expect(sent).toHaveLength(1);
      expect(sent[0]).toMatchObject({ width: 320, height: 240 });
      emit(treeView({ breadcrumbs: [] }));
      await flushPromises();
      emit({ event: "tree_view" });
      await flushPromises();
      sent.length = 0;
      resize(1000, 600);
      expect(lastRequest("tree_view")).toMatchObject({ root: "/", path: "/" });
    });
  });

  describe("breadcrumbs and inspector", () => {
    it("collapses long trails and opens crumb sibling menus", async () => {
      const deep = ["/", "/a", "/a/b", "/a/b/c", "/a/b/c/d", "/a/b/c/d/e"].map((path) => ({ path, label: path.split("/").at(-1) || "/" }));
      const siblings = [
        { name: "e", path: "/a/b/c/d/e", bytes: 100, is_dir: true, category: "Code" },
        { name: "f.txt", path: "/a/b/c/d/f.txt", bytes: 1, is_dir: false },
      ];
      deep[5] = { ...deep[5], siblings } as typeof deep[number];
      await openSpace(treeView({ breadcrumbs: deep }));
      expect(wrapper.findAll(".tree-crumb").map((crumb) => crumb.text())).toEqual(["/", "c", "d", "e"]);
      expect(wrapper.find(".tree-crumb-gap").exists()).toBe(true);
      await wrapper.find(".crumb-siblings").trigger("click");
      const options = wrapper.findAll(".crumb-menu button");
      expect(options).toHaveLength(2);
      expect(options[0].classes()).toContain("chosen");
      expect(options[1].find(".crumb-bar b").attributes("style")).toContain("width: 2%");
      await options[1].trigger("click");
      expect(lastRequest("tree_view")).toMatchObject({ root: "/", path: "/a/b/c/d/f.txt", open: false });
      expect(wrapper.find(".crumb-menu").exists()).toBe(false);
      await wrapper.find(".crumb-siblings").trigger("click");
      await wrapper.findAll(".crumb-menu button")[0].trigger("click");
      expect(lastRequest("tree_view")).toMatchObject({ path: "/a/b/c/d/e", open: true });
      await wrapper.find(".crumb-siblings").trigger("click");
      await wrapper.find(".crumb-siblings").trigger("click");
      expect(wrapper.find(".crumb-menu").exists()).toBe(false);
      await wrapper.find(".crumb-siblings").trigger("click");
      wrapper.find(".crumb-menu").element.dispatchEvent(new Event("pointerdown", { bubbles: true }));
      await flushPromises();
      expect(wrapper.find(".crumb-menu").exists()).toBe(true);
      document.body.dispatchEvent(new Event("pointerdown", { bubbles: true }));
      await flushPromises();
      expect(wrapper.find(".crumb-menu").exists()).toBe(false);
    });

    it("lists siblings, findings and selection facts in the inspector", async () => {
      const siblings = Array.from({ length: 10 }, (_, index) => ({ name: `s${index}`, path: `/Users/s${index}`, bytes: 1024, is_dir: false }));
      await openSpace(treeView({
        siblings,
        worth_a_look: [{ path: "/Users/cache", bytes: GB, reason: "Rebuildable cache" }],
        selection: { name: "big.bin", path: "/Users/big.bin", bytes: GB, share: 0.25, modified: 0, reason: "Disk image", marked: true },
        unreadable: 0,
      }));
      const inspector = wrapper.find(".tree-inspector");
      expect(inspector.text()).toContain("+2 more");
      expect(inspector.text()).toContain("25.0%");
      expect(inspector.text()).toContain("Disk image");
      expect(inspector.text()).toContain("UNMARK PATH");
      expect(inspector.text()).toContain("/cache");
      await inspector.findAll(".sibling-list button")[1].trigger("click");
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/s1", open: true });
      await inspector.find(".worth-a-look button").trigger("click");
      expect(lastRequest("tree_view")).toMatchObject({ path: "/Users/cache", open: false });
      emit(treeView({ selection: "none", tiles: "none", breadcrumbs: "none", siblings: "none", worth_a_look: "none", marks: "none" }));
      await flushPromises();
      expect(wrapper.find(".tree-inspector h2").text()).toBe("No selection");
      expect(wrapper.text()).toContain("Selected volume");
    });

    it("resizes the inspector by dragging, resets on double-click and hides it in narrow windows", async () => {
      await openSpace();
      const view = () => wrapper.find(".disk-view").attributes("style");
      const handle = wrapper.find(".inspector-resize").element;
      handle.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true, cancelable: true, clientX: 500 }));
      window.dispatchEvent(new MouseEvent("pointermove", { clientX: 400 }));
      await flushPromises();
      expect(view()).toContain("--inspector-w: 386px");
      window.dispatchEvent(new MouseEvent("pointermove", { clientX: -1000 }));
      await flushPromises();
      expect(view()).toContain("--inspector-w: 520px");
      window.dispatchEvent(new MouseEvent("pointerup"));
      window.dispatchEvent(new MouseEvent("pointermove", { clientX: 900 }));
      await flushPromises();
      expect(view()).toContain("--inspector-w: 520px");
      await wrapper.find(".inspector-resize").trigger("dblclick");
      expect(view()).toContain("--inspector-w: 286px");

      Object.defineProperty(window, "innerWidth", { value: 800, configurable: true, writable: true });
      window.dispatchEvent(new Event("resize"));
      await flushPromises();
      expect(wrapper.find(".tree-inspector").exists()).toBe(false);
      expect(wrapper.find(".disk-view").classes()).toContain("no-inspector");
      Object.defineProperty(window, "innerWidth", { value: 1024, configurable: true, writable: true });
      window.dispatchEvent(new Event("resize"));
    });

    it("focuses the treemap after toolbar controls and closes the filter on Enter", async () => {
      await openSpace();
      await press("/");
      const input = wrapper.find<HTMLInputElement>(".tree-filter input");
      await input.setValue("bin");
      await input.trigger("keydown.enter");
      await flushPromises();
      expect(document.activeElement).toBe(canvas());
      expect(input.element.value).toBe("bin");
      expect(wrapper.find(".tree-filter").isVisible()).toBe(true);
      await wrapper.find(".tree-controls").trigger("click");
      await (await button(wrapper, "AGE")).trigger("click");
      expect(document.activeElement).toBe(canvas());
    });
  });

  describe("review and removal", () => {
    const mark = { path: "/Users/big.bin", bytes: GB, is_dir: false, hidden: false };
    const reviewEvent = (extra: Record<string, unknown> = {}): CliEvent => ({
      event: "review", root: "/", marks: [mark, { ...mark, path: "/Users/dir", is_dir: true }], targets: 2,
      reclaim_bytes: 2 * GB, foreign_bytes: GB, blocked: [{ path: "/Users", reason: "home directory" }], ...extra,
    });

    async function openReview(extra: Record<string, unknown> = {}): Promise<void> {
      await openSpace(treeView({ marks: [mark] }));
      emit(reviewEvent(extra));
      await flushPromises();
    }

    it("lists marks and blocked paths, unmarks rows, clears marks and exports", async () => {
      Object.defineProperty(navigator, "clipboard", { value: { writeText: vi.fn(async () => { throw new Error("denied"); }) }, configurable: true });
      await openReview();
      const screen = wrapper.find(".review-workspace");
      expect(screen.text()).toContain("DIRECTORY");
      expect(screen.text()).toContain("BLOCKED · home directory");
      expect(screen.text()).toContain("1.0 GB");
      await screen.find("[aria-label='Unmark /Users/dir']").trigger("click");
      expect(lastRequest("toggle_mark")).toMatchObject({ path: "/Users/dir" });
      expect(sent.at(-1)?.command).toBe("review");

      await (await button(wrapper, "SAVE PATH LIST")).trigger("click");
      expect(lastRequest("export_marks")).toMatchObject({ format: "paths", save: true });
      emit({ event: "marks_export", text: "/Users/big.bin\n", saved_path: "/Users/me/karen-marks.txt" });
      await flushPromises();
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith("/Users/big.bin\n");
      expect(wrapper.text()).toContain("SAVED /Users/me/karen-marks.txt");
      await (await button(wrapper, "COPY AGENT PROMPT")).trigger("click");
      expect(lastRequest("export_marks")).toMatchObject({ format: "prompt", save: false });
      emit({ event: "marks_export", text: "prompt", format: "prompt" });
      await flushPromises();
      expect(wrapper.text()).toContain("COPIED AGENT PROMPT");
      emit({ event: "marks_export", format: "paths" });
      await flushPromises();
      expect(wrapper.text()).toContain("COPIED PATH LIST");

      await (await button(wrapper, "CLEAR MARKS")).trigger("click");
      expect(sent.slice(-2).map((request) => request.command)).toEqual(["clear_marks", "review"]);
      emit({ event: "marks_changed", root: "/", marks: [] });
      emit({ event: "review", root: "/", targets: 0 });
      await flushPromises();
      expect(wrapper.text()).toContain("NO MARKED PATHS TO REVIEW");
      expect(wrapper.find(".review-commit").attributes("disabled")).toBeDefined();
      emit({ event: "marks_changed" });
      await flushPromises();
    });

    it("cancels review with Escape and with the confirm dialog's Cancel", async () => {
      await openReview();
      await press("Escape");
      expect(lastRequest("cancel_review")).toBeDefined();
      expect(wrapper.find(".review-workspace").exists()).toBe(false);
      emit(reviewEvent());
      await flushPromises();
      await (await button(wrapper, "CONTINUE TO CONFIRMATION")).trigger("click");
      await (await button(wrapper, "CANCEL")).trigger("click");
      expect(wrapper.find("[role=dialog]").exists()).toBe(false);
      expect(wrapper.find(".review-workspace").exists()).toBe(false);
      emit(reviewEvent());
      await flushPromises();
      sent.length = 0;
      await (await button(wrapper, "Space")).trigger("click");
      expect(lastRequest("cancel_review")).toBeDefined();
      expect(wrapper.find(".tree-console").exists()).toBe(true);
    });

    it("repaints a fresh treemap after leaving Review and skips it when Review stays open off-tab", async () => {
      await openReview();
      await (await button(wrapper, "Runtime")).trigger("click");
      await wrapper.find(".runtime-volume").trigger("click");
      expect(wrapper.find(".review-workspace").exists()).toBe(true);
      await (await button(wrapper, "RETURN TO MAP")).trigger("click");
      await flushPromises();
      expect(wrapper.find("canvas.treemap-canvas").exists()).toBe(true);
      resize(700, 400);
      expect(lastRequest("tree_view")).toMatchObject({ width: 700, height: 400 });
      emit(reviewEvent());
      await flushPromises();
      await (await button(wrapper, "Runtime")).trigger("click");
      await (await button(wrapper, "Space")).trigger("click");
      expect(lastRequest("cancel_review")).toBeDefined();
    });

    it("confirms a permanent delete and reports the free-space gain", async () => {
      await openReview();
      await (await button(wrapper, "PERMANENT")).trigger("click");
      await (await button(wrapper, "CONTINUE TO CONFIRMATION")).trigger("click");
      expect(wrapper.find("[role=dialog]").text()).toContain("2 targets · 2.0 GB expected volume recovery");
      await (await button(wrapper, "CONFIRM PERMANENT DELETE")).trigger("click");
      expect(lastRequest("confirm_remove")).toMatchObject({ mode: "permanent" });
      await wrapper.find<HTMLInputElement>("input[value=trash]").exists();

      emit({ event: "removal_started", mode: "permanent" });
      emit({ event: "removal_progress" });
      emit({ event: "removal_item", path: "/Users/big.bin", ok: true });
      emit({ event: "removal_item", path: "/Users/dir", ok: false });
      await flushPromises();
      expect(wrapper.text()).toContain("2 / 2");
      expect(wrapper.text()).toContain("100%");
      expect(wrapper.text()).toContain("/Users/dir · removal error");
      expect(wrapper.find(".review-commit").attributes("disabled")).toBeDefined();
      expect(wrapper.text()).toContain("MEASURING");

      emit({ event: "removal_complete", removed: 1, bytes: GB, failed: 1 });
      await flushPromises();
      expect(lastRequest("volumes")).toBeDefined();
      emit({ event: "volumes", volumes: [{ ...volumes[0], available_bytes: 41 * GB }] });
      await flushPromises();
      expect(wrapper.text()).toContain("+1.0 GB");
      emit({ event: "volumes", volumes: [{ ...volumes[0], available_bytes: 1 * GB }] });
      await flushPromises();
      expect(wrapper.text()).toContain("+1.0 GB");
      expect(wrapper.text()).toContain("1 removed · 1.0 GB · 1 failed");
    });

    it("shows a free-space loss and zero progress for an empty removal", async () => {
      await openReview({ targets: undefined });
      emit({ event: "removal_started", mode: "trash" });
      await flushPromises();
      expect(wrapper.text()).toContain("0 / 0");
      expect(wrapper.find(".removal-progress i").attributes("style")).toContain("width: 0%");
      emit({ event: "removal_complete", removed: 0, failed: 0 });
      emit({ event: "volumes", volumes: [{ ...volumes[0], available_bytes: 39 * GB }] });
      await flushPromises();
      expect(wrapper.text()).toContain("−1.0 GB");
      await (await button(wrapper, "RETURN TO MAP")).trigger("click");
      expect(lastRequest("scan")?.root).toBe("/");
    });

    it("ignores removal events without a running removal", async () => {
      await openSpace();
      emit({ event: "removal_progress", total: 3 });
      emit({ event: "removal_item", ok: false });
      emit({ event: "removal_complete", removed: 0, bytes: 0, failed: 0 });
      await flushPromises();
      expect(wrapper.text()).toContain("REMOVED 0 · 0 B · 0 FAILED");
    });

    it("renders Battery and Network telemetry, Runtime mini-instruments, and tray navigation", async () => {
      host.trayStatus.length = 0;
      emit({
        event: "resources",
        cpu_percent: 24,
        memory_used_bytes: 8 * GB,
        memory_total_bytes: 16 * GB,
        cpu_processes_available: true,
        cpu_processes: [],
        memory_processes_available: true,
        memory_processes: [],
        battery: {
          present: true,
          state: "discharging",
          percent: 82,
          power_watts: 14.5,
          energy_wh: 49.2,
          energy_full_wh: 60.0,
          energy_design_wh: 64.0,
          health_percent: 94,
          voltage_volts: 12.15,
          temperature_celsius: 31.2,
          cycle_count: 110,
          time_to_empty_secs: 7500,
          time_to_full_secs: null,
        },
        network: {
          rx_bytes_per_sec: 2048,
          tx_bytes_per_sec: 1024,
          total_rx_bytes: 10 * GB,
          total_tx_bytes: 2 * GB,
          interfaces: [
            {
              name: "en0",
              rx_bytes_per_sec: 2048,
              tx_bytes_per_sec: 1024,
              total_rx_bytes: 10 * GB,
              total_tx_bytes: 2 * GB,
              packets_rx: 100,
              packets_tx: 50,
              errors_rx: 0,
              errors_tx: 1,
            },
          ],
        },
      });
      emit({
        event: "resources",
        cpu_percent: 28,
        memory_used_bytes: 8 * GB,
        memory_total_bytes: 16 * GB,
        battery: { present: true, percent: 81, power_watts: 15.0 },
        network: { rx_bytes_per_sec: 4096, tx_bytes_per_sec: 2048, total_rx_bytes: 10 * GB, total_tx_bytes: 2 * GB, interfaces: [] },
      });
      await flushPromises();
      expect(host.trayStatus.at(-1)).toContain("BAT 81%");

      await wrapper.find(".runtime-battery-card").trigger("click");
      expect(wrapper.find(".battery-view").exists()).toBe(true);
      expect(wrapper.text()).toContain("81%");

      emit({
        event: "resources",
        cpu_percent: 10,
        memory_used_bytes: 4 * GB,
        memory_total_bytes: 16 * GB,
        battery: {
          present: true,
          state: "charging",
          percent: 85,
          power_watts: 30.0,
          energy_wh: 51.0,
          energy_full_wh: 60.0,
          energy_design_wh: 64.0,
          health_percent: 94,
          voltage_volts: 12.4,
          temperature_celsius: 32.0,
          cycle_count: 110,
          time_to_empty_secs: null,
          time_to_full_secs: 3660,
        },
      });
      await flushPromises();
      expect(wrapper.text()).toContain("1h 1m");
      expect(wrapper.text()).toContain("51.0 Wh");
      expect(wrapper.text()).toContain("94%");

      emit({
        event: "resources",
        cpu_percent: 10,
        memory_used_bytes: 4 * GB,
        memory_total_bytes: 16 * GB,
        battery: { present: false },
      });
      await flushPromises();
      expect(wrapper.text()).toContain("NO BATTERY DETECTED (AC POWER)");

      await (await button(wrapper, "Runtime")).trigger("click");
      await wrapper.find(".runtime-network-card").trigger("click");
      expect(wrapper.find(".network-view").exists()).toBe(true);
      expect(wrapper.text()).toContain("NO NETWORK INTERFACES DETECTED");

      emit({
        event: "resources",
        cpu_percent: 12,
        memory_used_bytes: 4 * GB,
        memory_total_bytes: 16 * GB,
        network: {
          rx_bytes_per_sec: 2048,
          tx_bytes_per_sec: 1024,
          total_rx_bytes: 1024,
          total_tx_bytes: 512,
          interfaces: [
            {
              name: "en0",
              rx_bytes_per_sec: 2048,
              tx_bytes_per_sec: 1024,
              total_rx_bytes: 1024,
              total_tx_bytes: 512,
              packets_rx: 10,
              packets_tx: 5,
              errors_rx: 0,
              errors_tx: 0,
            },
          ],
        },
      });
      await flushPromises();
      expect(wrapper.text()).toContain("en0");
      expect(wrapper.text()).toContain("2.0 KB/s");

      host.emitTray("battery");
      await flushPromises();
      expect(wrapper.find(".battery-view").exists()).toBe(true);
      host.emitTray("space");
      await flushPromises();
      expect(wrapper.find(".disk-view").exists()).toBe(true);
      host.emitTray("unknown");
      await flushPromises();
      expect(wrapper.find(".disk-view").exists()).toBe(true);
      expect(host.trayStatus.some((status) => status.includes("CPU 12%  MEM 25%"))).toBe(true);
    });
  });
});
