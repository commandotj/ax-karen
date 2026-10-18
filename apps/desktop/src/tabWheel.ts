export type TabId = "runtime" | "cpu" | "memory" | "battery" | "network" | "disk";

export type TabItem = {
  id: TabId;
  index: string;
  label: string;
  badge: string;
};

export function wheelOffset(activeIndex: number, index: number, total: number): number {
  const half = Math.floor(total / 2);
  return ((index - activeIndex + half + total) % total) - half;
}

export function stepTab<T>(ids: readonly T[], active: T, delta: -1 | 1): T {
  const at = Math.max(0, ids.indexOf(active));
  return ids[(at + delta + ids.length) % ids.length];
}

export function wheelDeltaStep(deltaX: number, deltaY: number): -1 | 0 | 1 {
  const primary = Math.abs(deltaX) > Math.abs(deltaY) ? deltaX : deltaY;
  if (Math.abs(primary) < 4) return 0;
  return primary > 0 ? 1 : -1;
}

export function wheelStyle(offset: number): Record<string, string> {
  const abs = Math.abs(offset);
  const shift = offset * 148 - Math.sign(offset) * Math.max(0, abs - 1) * 18;
  const angle = -offset * 22;
  const scale = Math.max(0.72, 1 - abs * 0.11);
  return {
    transform: `translate(-50%, -50%) perspective(760px) translateX(${shift}px) rotateY(${angle}deg) scale(${scale.toFixed(2)})`,
    opacity: abs > 2 ? "0" : (1 - abs * 0.28).toFixed(2),
    zIndex: String(10 - abs),
    pointerEvents: abs > 2 ? "none" : "auto",
  };
}
