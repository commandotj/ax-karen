import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import TabWheel from "./TabWheel.vue";
import { formatTraySummary, formatTrayTitle, stepTab, wheelDeltaStep, wheelOffset, wheelStyle, type TabItem } from "./tabWheel";

const sampleTabs: TabItem[] = [
  { id: "runtime", index: "01", label: "Runtime", badge: "LIVE" },
  { id: "cpu", index: "02", label: "CPU", badge: "LIVE" },
  { id: "memory", index: "03", label: "Memory", badge: "LIVE" },
  { id: "battery", index: "04", label: "Battery", badge: "84%" },
  { id: "network", index: "05", label: "Network", badge: "LIVE" },
  { id: "disk", index: "06", label: "Space", badge: "OPEN" },
];

describe("tabWheel pure functions", () => {
  it("computes shortest circular offset around the active tab", () => {
    expect(wheelOffset(0, 0, 6)).toBe(0);
    expect(wheelOffset(0, 1, 6)).toBe(1);
    expect(wheelOffset(0, 5, 6)).toBe(-1);
    expect(wheelOffset(5, 0, 6)).toBe(1);
    expect(wheelOffset(3, 1, 6)).toBe(-2);
  });

  it("steps forward and backward circularly", () => {
    const ids = sampleTabs.map((item) => item.id);
    expect(stepTab(ids, "runtime", -1)).toBe("disk");
    expect(stepTab(ids, "disk", 1)).toBe("runtime");
    expect(stepTab(ids, "battery", 1)).toBe("network");
  });

  it("maps wheel deltas to discrete steps with deadzone", () => {
    expect(wheelDeltaStep(0, 0)).toBe(0);
    expect(wheelDeltaStep(2, -3)).toBe(0);
    expect(wheelDeltaStep(0, 24)).toBe(1);
    expect(wheelDeltaStep(0, -24)).toBe(-1);
    expect(wheelDeltaStep(30, 5)).toBe(1);
    expect(wheelDeltaStep(-30, 5)).toBe(-1);
  });

  it("computes 3D rotary drum style from offset", () => {
    const center = wheelStyle(0);
    expect(center.opacity).toBe("1.00");
    expect(center.zIndex).toBe("10");
    expect(center.pointerEvents).toBe("auto");

    const far = wheelStyle(3);
    expect(far.opacity).toBe("0");
    expect(far.pointerEvents).toBe("none");
  });

  it("formats tray title and summary side by side", () => {
    expect(formatTrayTitle(null, null)).toBe("CPU —%  MEM —%");
    expect(formatTrayTitle(12.4, 45.8)).toBe("CPU 12%  MEM 46%");
    expect(formatTraySummary(null, null)).toBe("Karen · CPU —% · MEM —%");
    expect(formatTraySummary(15.2, 50, true, 85)).toBe("Karen · CPU 15.2% · MEM 50% · BAT 85%");
    expect(formatTraySummary(15.2, 50, false, null)).toBe("Karen · CPU 15.2% · MEM 50%");
  });
});

describe("TabWheel component", () => {
  it("emits select on tab click, stepper click, and wheel events", async () => {
    const wrapper = mount(TabWheel, {
      props: {
        tabs: sampleTabs,
        active: "battery",
      },
    });

    await wrapper.find(".wheel-prev").trigger("click");
    expect(wrapper.emitted("select")?.[0]).toEqual(["memory"]);

    await wrapper.find(".wheel-next").trigger("click");
    expect(wrapper.emitted("select")?.[1]).toEqual(["network"]);

    await wrapper.find(".tab-wheel").trigger("wheel", { deltaX: 0, deltaY: 20 });
    expect(wrapper.emitted("select")?.[2]).toEqual(["network"]);

    await wrapper.find(".tab-wheel").trigger("wheel", { deltaX: 0, deltaY: 1 });
    expect(wrapper.emitted("select")?.length).toBe(3);

    const cpuBtn = wrapper.findAll(".wheel-tab").find((btn) => btn.text().includes("CPU"));
    await cpuBtn?.trigger("click");
    expect(wrapper.emitted("select")?.[3]).toEqual(["cpu"]);
  });
});
