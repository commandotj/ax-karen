import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DownloadEvent } from "@tauri-apps/plugin-updater";

const plugin = vi.hoisted(() => ({
  tauri: true,
  check: vi.fn(),
  relaunch: vi.fn(),
  getVersion: vi.fn(),
  unlisten: vi.fn(),
  listen: vi.fn(),
  logInfo: vi.fn(),
  logError: vi.fn(),
  trigger: (): void => undefined,
}));

vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => plugin.tauri }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: plugin.check }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: plugin.relaunch }));
vi.mock("@tauri-apps/plugin-log", () => ({ info: plugin.logInfo, error: plugin.logError }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: plugin.getVersion }));
vi.mock("@tauri-apps/api/event", () => ({ listen: plugin.listen }));

import UpdatePrompt from "./UpdatePrompt.vue";

function pending(version = "0.13.0") {
  return {
    version,
    downloadAndInstall: vi.fn<(onEvent?: (event: DownloadEvent) => void) => Promise<void>>(async () => undefined),
  };
}

class PrivateFieldUpdate {
  #rid = 17;
  version = "0.14.0";
  downloads = 0;

  async downloadAndInstall(): Promise<void> {
    if (this.#rid !== 17) throw new Error("invalid update resource");
    this.downloads += 1;
  }
}

async function open(found: { version: string; downloadAndInstall: (onEvent?: (event: DownloadEvent) => void) => Promise<void> } | null) {
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
    plugin.logInfo.mockReset().mockResolvedValue(undefined);
    plugin.logError.mockReset().mockResolvedValue(undefined);
    plugin.listen.mockReset().mockImplementation(async (_event: string, handler: () => void) => {
      plugin.trigger = handler;
      return plugin.unlisten;
    });
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
    expect(plugin.logError).toHaveBeenCalledWith(expect.stringContaining("phase=check-failed"));
  });

  it.each([
    ["string", "offline", "error=offline"],
    ["undefined", undefined, "error=undefined"],
    ["object", { code: "offline" }, 'error={"code":"offline"}'],
  ])("logs non-Error check failures when cause is %s", async (_kind, cause, expected) => {
    plugin.check.mockRejectedValue(cause);
    mount(UpdatePrompt);
    await flushPromises();
    expect(plugin.logError).toHaveBeenCalledWith(expect.stringContaining(expected as string));
  });

  it("falls back to a safe string when a failure cause cannot be serialized", async () => {
    const cause: { self?: unknown } = {};
    cause.self = cause;
    plugin.check.mockRejectedValue(cause);
    mount(UpdatePrompt);
    await flushPromises();
    expect(plugin.logError).toHaveBeenCalledWith(expect.stringContaining("error=[object Object]"));
  });

  it("keeps updater usable when writing an error log fails", async () => {
    plugin.check.mockRejectedValue(new Error("offline"));
    plugin.logError.mockRejectedValueOnce(new Error("logger unavailable"));
    const wrapper = mount(UpdatePrompt);
    await flushPromises();
    expect(wrapper.find(".update-prompt").exists()).toBe(false);
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

  it("preserves the updater resource instance when installing", async () => {
    const found = new PrivateFieldUpdate();
    const wrapper = await open(found);

    await wrapper.find(".update-install").trigger("click");
    await flushPromises();

    expect(found.downloads).toBe(1);
    expect(wrapper.find(".update-state").text()).toContain("Update is installed");
    expect(plugin.relaunch).toHaveBeenCalledTimes(1);
  });

  it("shows download progress and moves through install and restart phases", async () => {
    const found = pending();
    let send: ((event: DownloadEvent) => void) | undefined;
    let finish: () => void = () => undefined;
    found.downloadAndInstall.mockImplementation(async (onEvent) => {
      onEvent?.({ event: "Started", data: { contentLength: 10 } });
      onEvent?.({ event: "Progress", data: { chunkLength: 5 } });
      send = onEvent;
      await new Promise<void>((resolve) => { finish = resolve; });
    });
    const wrapper = await open(found);

    await wrapper.find(".update-install").trigger("click");
    expect(wrapper.find(".update-state").text()).toContain("50%");
    expect(wrapper.find('[role="progressbar"]').attributes("aria-valuenow")).toBe("50");
    send?.({ event: "Finished" });
    await wrapper.vm.$nextTick();
    expect(wrapper.find(".update-state").text()).toContain("Installing");
    finish();
    await flushPromises();

    expect(wrapper.find(".update-state").text()).toContain("installed");
    expect(wrapper.find(".update-install").text()).toBe("Restart Karen");
  });

  it("keeps download progress indeterminate when the server omits content length", async () => {
    const found = pending();
    found.downloadAndInstall.mockImplementation((onEvent) => {
      onEvent?.({ event: "Started", data: { contentLength: undefined } });
      onEvent?.({ event: "Progress", data: { chunkLength: 8 } });
      return new Promise<void>(() => undefined);
    });
    const wrapper = await open(found);
    await wrapper.find(".update-install").trigger("click");
    await flushPromises();
    expect(wrapper.find('[role="progressbar"]').attributes("aria-valuenow")).toBeUndefined();
    expect(wrapper.find(".update-state").text()).toContain("Downloading");
  });

  it("reports restart failure as installed and keeps raw diagnostics out of the UI", async () => {
    const found = pending();
    plugin.relaunch.mockRejectedValueOnce(new Error("restart failed Authorization: Bearer secret https://example.com/a?token=secret"));
    const wrapper = await open(found);

    await wrapper.find(".update-install").trigger("click");
    await flushPromises();

    expect(wrapper.find(".update-state").text()).toContain("Update is installed");
    expect(wrapper.text()).not.toContain("restart failed");
    expect(wrapper.text()).not.toContain("secret");
    expect(wrapper.text()).not.toContain(".karen/logs");
    expect(plugin.logError).toHaveBeenCalledWith(expect.stringContaining("?<redacted>"));
    expect(plugin.logError.mock.calls.at(-1)?.[0]).not.toContain("secret");
  });

  it("keeps installed state after a failed manual restart retry", async () => {
    const found = pending();
    plugin.relaunch.mockRejectedValueOnce(new Error("restart unavailable"));
    plugin.relaunch.mockResolvedValueOnce(undefined);
    const wrapper = await open(found);
    await wrapper.find(".update-install").trigger("click");
    await flushPromises();
    await wrapper.find(".update-install").trigger("click");
    await flushPromises();
    expect(plugin.relaunch).toHaveBeenCalledTimes(2);
    expect(found.downloadAndInstall).toHaveBeenCalledTimes(1);
    expect(wrapper.find(".update-state").text()).toContain("Update is installed");
  });

  it("keeps installed state when manual restart retry also fails", async () => {
    const found = pending();
    plugin.relaunch.mockRejectedValue(new Error("restart unavailable"));
    const wrapper = await open(found);
    await wrapper.find(".update-install").trigger("click");
    await flushPromises();
    await wrapper.find(".update-install").trigger("click");
    await flushPromises();
    expect(plugin.relaunch).toHaveBeenCalledTimes(2);
    expect(found.downloadAndInstall).toHaveBeenCalledTimes(1);
    expect(wrapper.find(".update-state").text()).toContain("Update is installed");
  });

  it("reports a failed install and allows another try", async () => {
    const found = pending();
    found.downloadAndInstall.mockImplementationOnce(async (onEvent) => {
      onEvent?.({ event: "Finished" });
      throw new Error("install failed");
    });
    const wrapper = await open(found);
    await wrapper.find(".update-install").trigger("click");
    await flushPromises();
    expect(plugin.relaunch).not.toHaveBeenCalled();
    expect(wrapper.find(".update-state").text()).toContain("could not be installed");
    expect(wrapper.find(".update-install").attributes("disabled")).toBeUndefined();
    await wrapper.find(".update-install").trigger("click");
    await flushPromises();
    expect(plugin.relaunch).toHaveBeenCalledTimes(1);
    expect(wrapper.find(".update-state").text()).toContain("Update is installed");
    expect(plugin.logError).toHaveBeenCalledWith(expect.stringContaining("phase=install-failed"));
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
      expect(plugin.logError).toHaveBeenCalledWith(expect.stringContaining("phase=check-failed"));
      expect(wrapper.find(".update-retry").exists()).toBe(true);
      plugin.check.mockResolvedValue(pending("0.14.0"));
      await wrapper.find(".update-retry").trigger("click");
      await flushPromises();
      expect(wrapper.find(".update-prompt").text()).toContain("0.14.0");
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
      expect(plugin.logInfo).toHaveBeenCalledWith(expect.stringContaining("phase=checking"));
    });
  });
});
