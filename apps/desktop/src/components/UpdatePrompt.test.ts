import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";

const plugin = vi.hoisted(() => ({
  tauri: true,
  check: vi.fn(),
  relaunch: vi.fn(),
  getVersion: vi.fn(),
  unlisten: vi.fn(),
  listen: vi.fn(),
  trigger: (): void => undefined,
}));

vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => plugin.tauri }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: plugin.check }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: plugin.relaunch }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: plugin.getVersion }));
vi.mock("@tauri-apps/api/event", () => ({ listen: plugin.listen }));

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
    plugin.getVersion.mockReset().mockResolvedValue("0.12.0");
    plugin.unlisten.mockReset();
    plugin.listen.mockReset().mockImplementation(async (_event: string, handler: () => void) => {
      plugin.trigger = handler;
      return plugin.unlisten;
    });
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

  describe("manual check from the tray menu", () => {
    async function manual(found: ReturnType<typeof pending> | null) {
      const wrapper = await open(null);
      plugin.check.mockClear();
      plugin.check.mockResolvedValue(found);
      plugin.trigger();
      await flushPromises();
      return wrapper;
    }

    it("listens for the tray event and stops listening on unmount", async () => {
      const wrapper = await open(null);
      expect(plugin.listen).toHaveBeenCalledWith("tray-check-update", expect.any(Function));
      wrapper.unmount();
      expect(plugin.unlisten).toHaveBeenCalledTimes(1);
    });

    it("does not listen outside Tauri", async () => {
      plugin.tauri = false;
      await open(null);
      expect(plugin.listen).not.toHaveBeenCalled();
    });

    it("shows the install prompt when an update exists", async () => {
      const wrapper = await manual(pending("0.14.0"));
      expect(plugin.check).toHaveBeenCalledTimes(1);
      expect(wrapper.find(".update-prompt").text()).toContain("0.14.0");
    });

    it("says Karen is up to date, with the running version, and dismisses", async () => {
      const wrapper = await manual(null);
      expect(wrapper.find(".update-notice").text()).toContain("0.12.0");
      expect(wrapper.find(".update-install").exists()).toBe(false);
      await wrapper.find(".update-dismiss").trigger("click");
      expect(wrapper.find(".update-notice").exists()).toBe(false);
    });

    it("says the check failed when it cannot complete", async () => {
      const wrapper = await open(null);
      plugin.check.mockRejectedValue(new Error("offline"));
      plugin.trigger();
      await flushPromises();
      expect(wrapper.find(".update-notice").exists()).toBe(true);
      expect(wrapper.find(".update-notice").text()).not.toContain("0.12.0");
      expect(console.warn).toHaveBeenCalled();
    });

    it("replaces an older notice with the result of the next check", async () => {
      const wrapper = await manual(null);
      expect(wrapper.find(".update-notice").exists()).toBe(true);
      plugin.check.mockResolvedValue(pending("0.14.0"));
      plugin.trigger();
      await flushPromises();
      expect(wrapper.find(".update-notice").exists()).toBe(false);
      expect(wrapper.find(".update-prompt").text()).toContain("0.14.0");
    });

    it("ignores the event while a check is running", async () => {
      const wrapper = await open(null);
      plugin.check.mockClear();
      let finish: (value: null) => void = () => undefined;
      plugin.check.mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
      plugin.trigger();
      plugin.trigger();
      expect(plugin.check).toHaveBeenCalledTimes(1);
      finish(null);
      await flushPromises();
      expect(wrapper.find(".update-notice").exists()).toBe(true);
    });

    it("ignores the event while an install is running", async () => {
      const found = pending();
      found.downloadAndInstall.mockImplementation(() => new Promise<void>(() => undefined));
      const wrapper = await open(found);
      await wrapper.find(".update-install").trigger("click");
      plugin.check.mockClear();
      plugin.trigger();
      await flushPromises();
      expect(plugin.check).not.toHaveBeenCalled();
    });

    it("keeps the startup check silent", async () => {
      const wrapper = await open(null);
      expect(wrapper.find(".update-notice").exists()).toBe(false);
      expect(plugin.getVersion).not.toHaveBeenCalled();
    });
  });
});
