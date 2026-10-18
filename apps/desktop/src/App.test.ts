import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CliEvent } from "./cli";

const sent: Record<string, unknown>[] = [];
let emit: (event: CliEvent) => void = () => undefined;

vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ startDragging() {}, minimize() {}, toggleMaximize() {}, close() {} }),
}));
vi.mock("./cli", () => ({
  KarenCli: class {
    constructor(onEvent: (event: CliEvent) => void) {
      emit = onEvent;
    }
    async start() {}
    async send(request: Record<string, unknown>) {
      sent.push(request);
    }
    async stop() {}
  },
}));

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
    HTMLCanvasElement.prototype.getContext = () => null;
    globalThis.ResizeObserver = class { observe() {} unobserve() {} disconnect() {} };
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
});
