import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";

const plugin = vi.hoisted(() => ({
  tauri: true,
  check: vi.fn(),
  relaunch: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => plugin.tauri }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: plugin.check }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: plugin.relaunch }));

import UpdatePrompt from "./UpdatePrompt.vue";

function pending(version = "0.13.0") {
  return { version, downloadAndInstall: vi.fn<() => Promise<void>>(async () => undefined) };
}

async function open(found: ReturnType<typeof pending> | null) {
  plugin.check.mockResolvedValue(found);
  const wrapper = mount(UpdatePrompt);
  await flushPromises();
  return wrapper;
}

describe("UpdatePrompt", () => {
  beforeEach(() => {
    plugin.tauri = true;
    plugin.check.mockReset();
    plugin.relaunch.mockReset();
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
  });

  it("does nothing outside Tauri", async () => {
    plugin.tauri = false;
    const wrapper = await open(pending());
    expect(plugin.check).not.toHaveBeenCalled();
    expect(wrapper.find(".update-prompt").exists()).toBe(false);
  });

  it("shows nothing when the app is up to date", async () => {
    const wrapper = await open(null);
    expect(plugin.check).toHaveBeenCalledTimes(1);
    expect(wrapper.find(".update-prompt").exists()).toBe(false);
  });

  it("stays silent when the check fails", async () => {
    plugin.check.mockRejectedValue(new Error("offline"));
    const wrapper = mount(UpdatePrompt);
    await flushPromises();
    expect(wrapper.find(".update-prompt").exists()).toBe(false);
    expect(console.warn).toHaveBeenCalled();
  });

  it("offers a newer version and hides on Later without installing", async () => {
    const found = pending("0.13.0");
    const wrapper = await open(found);
    expect(wrapper.find(".update-prompt").text()).toContain("0.13.0");
    await wrapper.find(".update-later").trigger("click");
    expect(wrapper.find(".update-prompt").exists()).toBe(false);
    expect(found.downloadAndInstall).not.toHaveBeenCalled();
    expect(plugin.relaunch).not.toHaveBeenCalled();
  });

  it("installs, then restarts, only after the user confirms", async () => {
    const found = pending();
    let finish: () => void = () => undefined;
    found.downloadAndInstall.mockImplementation(() => new Promise<void>((resolve) => { finish = resolve; }));
    const wrapper = await open(found);
    expect(found.downloadAndInstall).not.toHaveBeenCalled();
    await wrapper.find(".update-install").trigger("click");
    expect(found.downloadAndInstall).toHaveBeenCalledTimes(1);
    expect(wrapper.find(".update-install").attributes("disabled")).toBeDefined();
    expect(plugin.relaunch).not.toHaveBeenCalled();
    finish();
    await flushPromises();
    expect(plugin.relaunch).toHaveBeenCalledTimes(1);
  });

  it("reports a failed install and allows another try", async () => {
    const found = pending();
    found.downloadAndInstall.mockRejectedValueOnce(new Error("bad signature"));
    const wrapper = await open(found);
    await wrapper.find(".update-install").trigger("click");
    await flushPromises();
    expect(plugin.relaunch).not.toHaveBeenCalled();
    expect(wrapper.find(".update-error").exists()).toBe(true);
    expect(wrapper.find(".update-install").attributes("disabled")).toBeUndefined();
    await wrapper.find(".update-install").trigger("click");
    await flushPromises();
    expect(plugin.relaunch).toHaveBeenCalledTimes(1);
    expect(wrapper.find(".update-error").exists()).toBe(false);
  });
});
