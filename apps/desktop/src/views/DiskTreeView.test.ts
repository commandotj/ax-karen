import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import DiskTreeView from "./DiskTreeView.vue";

describe("DiskTreeView review-summary and inspector", () => {
  it("renders review summary when review is provided with targets", async () => {
    const wrapper = mount(DiskTreeView, {
      props: {
        selectedVolume: "/",
        volumes: [{ mount_point: "/" }],
        treeRoot: "/",
        tiles: [],
        files: 10,
        dirs: 2,
        bytes: 1024,
        scanProgress: { files: 10, dirs: 2, bytes: 1024 },
        error: "",
        scanning: false,
        previewing: false,
        workers: 4,
        power: "balanced",
        unreadable: 0,
        exportStatus: "",
        removal: null,
        siblings: [],
        findings: [],
        marks: [{ path: "/tmp/old", bytes: 512, is_dir: false, hidden: false }],
        selection: { path: "/tmp/old", name: "old", bytes: 512, category: "other", marked: true },
        review: {
          targets: 1,
          reclaim_bytes: 512,
          blocked: [{ path: "/etc", reason: "system protected" }],
        },
        reviewScreen: false,
        breadcrumbs: [{ path: "/", label: "/" }],
        removalConfirm: false,
        removalMode: "trash",
        removalStatus: "",
        isMacOS: true,
        treeMode: "size",
        treeDepth: 4,
        includeHidden: true,
        apparentSize: false,
      },
    });

    const summary = wrapper.find(".review-summary");
    expect(summary.exists()).toBe(true);
    expect(summary.text()).toContain("1 targets");
    expect(summary.text()).toContain("BLOCKED · /etc · system protected");

    const continueBtn = summary.findAll("button")[0];
    await continueBtn.trigger("click");
    expect(wrapper.emitted("update:removalConfirm")?.[0]).toEqual([true]);

    const cancelBtn = summary.findAll("button")[1];
    await cancelBtn.trigger("click");
    expect(wrapper.emitted("cancelReview")).toBeTruthy();
  });
});
