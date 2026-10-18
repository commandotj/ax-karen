import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import DataTable from "./DataTable.vue";

describe("DataTable", () => {
  it("renders columns and rows with custom slots", () => {
    const columns = ["RANK / PROCESS", "PID", "CPU SHARE"];
    const items = [
      { pid: 101, name: "karen-core", cpu: 12.5 },
      { pid: 202, name: "window-server", cpu: 5.1 },
    ];

    const wrapper = mount(DataTable, {
      props: {
        columns,
        items,
        sectionTitle: "ACTIVE PROCESSES",
        sectionSubtitle: "TOP 2",
        headingClass: "custom-heading",
        rowClass: "custom-row",
        keyProp: (item: Record<string, any>) => item.pid,
      },
      slots: {
        row: `<template #row="{ item, index }">
          <span class="detail-process-identity"><i>{{ index + 1 }}</i><b>{{ item.name }}</b></span>
          <span class="process-pid">{{ item.pid }}</span>
          <span class="detail-process-reading"><b>{{ item.cpu }}%</b></span>
        </template>`,
      },
    });

    expect(wrapper.find(".instrument-section-heading").text()).toContain("ACTIVE PROCESSES");
    expect(wrapper.find(".instrument-section-heading").text()).toContain("TOP 2");
    expect(wrapper.findAll(".detail-table-heading span")).toHaveLength(3);
    expect(wrapper.findAll(".detail-process-row")).toHaveLength(2);
    expect(wrapper.find(".detail-process-row").classes()).toContain("custom-row");
    expect(wrapper.text()).toContain("karen-core");
    expect(wrapper.text()).toContain("101");
  });

  it("renders default empty message when items are empty", () => {
    const wrapper = mount(DataTable, {
      props: {
        columns: ["COL1", "COL2"],
        items: [],
      },
    });

    expect(wrapper.find(".process-empty").text()).toBe("NO READINGS AVAILABLE");
  });

  it("renders custom empty message or empty slot when empty prop is true", () => {
    const wrapperWithMessage = mount(DataTable, {
      props: {
        columns: ["COL1"],
        items: [],
        empty: true,
        emptyMessage: "NO INTERFACES DETECTED",
      },
    });
    expect(wrapperWithMessage.find(".process-empty").text()).toBe("NO INTERFACES DETECTED");

    const wrapperWithSlot = mount(DataTable, {
      props: {
        columns: ["COL1"],
        items: [{ id: 1 }],
        empty: true,
      },
      slots: {
        empty: `<p class="custom-empty-slot">CUSTOM EMPTY</p>`,
      },
    });
    expect(wrapperWithSlot.find(".custom-empty-slot").text()).toBe("CUSTOM EMPTY");
  });
});
