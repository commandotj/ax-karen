import type { Volume } from "./types";

export function formatBytes(value: number): string {
  if (value < 1024) return `${value} B`;
  const units = ["KB", "MB", "GB", "TB", "PB"];
  let amount = value;
  let index = -1;
  do {
    amount /= 1024;
    index += 1;
  } while (amount >= 1024 && index < units.length - 1);
  return `${amount.toFixed(1)} ${units[index]}`;
}

export function formatRate(value: number | null | undefined): string {
  return typeof value === "number" ? `${formatBytes(value)}/s` : "—";
}

export function formatDuration(secs: number | null | undefined): string {
  if (typeof secs !== "number" || secs <= 0) return "—";
  return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
}

export function chartPath(samples: number[], maximum: number): string {
  if (samples.length < 2) return "";
  const safeMax = maximum <= 0 ? 1 : maximum;
  return samples
    .map((value, index) => {
      const x = (index / (samples.length - 1)) * 1000;
      const y = 100 - Math.min(value / safeMax, 1) * 100;
      return `${index === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

export function volumePercent(volume: Volume): number {
  const total = volume.total_bytes ?? 0;
  const free = volume.free_bytes ?? 0;
  return total > 0 ? Math.min(100, Math.max(0, ((total - free) / total) * 100)) : 0;
}

export function meterStyle(value: number): Record<string, string> {
  return { "--reading": `${Math.max(0, Math.min(value, 100))}%` };
}

export function processMetricWidth(value: number, largest: number): string {
  return `${largest > 0 ? Math.min((value / largest) * 100, 100) : 0}%`;
}
