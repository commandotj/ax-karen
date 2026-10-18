import { describe, expect, it } from "vitest";
import {
  chartPath,
  formatBytes,
  formatDuration,
  formatRate,
  meterStyle,
  processMetricWidth,
  volumePercent,
} from "./format";
import { POWER_PRESETS } from "./types";

describe("formatBytes", () => {
  it("formats bytes below 1024 as B", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1023)).toBe("1023 B");
  });

  it("formats multiples of 1024 with units", () => {
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(1024 * 1024)).toBe("1.0 MB");
    expect(formatBytes(1024 * 1024 * 1024)).toBe("1.0 GB");
    expect(formatBytes(1024 * 1024 * 1024 * 1024)).toBe("1.0 TB");
    expect(formatBytes(1024 * 1024 * 1024 * 1024 * 1024)).toBe("1.0 PB");
    expect(formatBytes(1024 * 1024 * 1024 * 1024 * 1024 * 2048)).toBe("2048.0 PB");
  });
});

describe("formatRate", () => {
  it("formats number values as bytes per second", () => {
    expect(formatRate(2048)).toBe("2.0 KB/s");
  });

  it("returns dash for null or undefined", () => {
    expect(formatRate(null)).toBe("—");
    expect(formatRate(undefined)).toBe("—");
  });
});

describe("formatDuration", () => {
  it("formats positive seconds into hours and minutes", () => {
    expect(formatDuration(3660)).toBe("1h 1m");
    expect(formatDuration(7200)).toBe("2h 0m");
  });

  it("returns dash for non-positive or non-number seconds", () => {
    expect(formatDuration(0)).toBe("—");
    expect(formatDuration(-10)).toBe("—");
    expect(formatDuration(null)).toBe("—");
    expect(formatDuration(undefined)).toBe("—");
  });
});

describe("chartPath", () => {
  it("returns empty string when samples have less than 2 points", () => {
    expect(chartPath([], 100)).toBe("");
    expect(chartPath([50], 100)).toBe("");
  });

  it("constructs SVG path commands for multiple samples", () => {
    const path = chartPath([0, 50, 100], 100);
    expect(path).toBe("M0.0,100.0 L500.0,50.0 L1000.0,0.0");
  });

  it("safely handles non-positive maximum", () => {
    const path = chartPath([0, 10], 0);
    expect(path).toContain("M0.0");
  });
});

describe("volumePercent", () => {
  it("calculates percentage used correctly", () => {
    expect(volumePercent({ total_bytes: 1000, free_bytes: 200, mount_point: "/" })).toBe(80);
    expect(volumePercent({ total_bytes: 0, free_bytes: 0, mount_point: "/" })).toBe(0);
    expect(volumePercent({ total_bytes: null, free_bytes: null, mount_point: "/" })).toBe(0);
  });
});

describe("meterStyle", () => {
  it("clamps reading between 0 and 100 percent", () => {
    expect(meterStyle(50)).toEqual({ "--reading": "50%" });
    expect(meterStyle(-10)).toEqual({ "--reading": "0%" });
    expect(meterStyle(120)).toEqual({ "--reading": "100%" });
  });
});

describe("processMetricWidth", () => {
  it("computes relative width capped at 100%", () => {
    expect(processMetricWidth(50, 100)).toBe("50%");
    expect(processMetricWidth(150, 100)).toBe("100%");
    expect(processMetricWidth(50, 0)).toBe("0%");
  });
});

describe("types runtime exports", () => {
  it("exports power preset list", () => {
    expect(POWER_PRESETS).toContain("balanced");
    expect(POWER_PRESETS.length).toBe(4);
  });
});
