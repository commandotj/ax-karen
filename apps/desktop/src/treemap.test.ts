import { describe, expect, it } from "vitest";
import { categoryColor, drawTreemap, tileAt, type TreeTile } from "./treemap";

type Call = [string, ...unknown[]];

// Records every method call and property write so paint order and styles can be asserted.
function fakeCanvas(): { canvas: HTMLCanvasElement; calls: Call[] } {
  const calls: Call[] = [];
  const state: Record<string, unknown> = { lineWidth: 1 };
  const context = new Proxy(state, {
    get(target, key: string) {
      if (key in target) return target[key];
      return (...args: unknown[]) => {
        calls.push([key, ...args]);
        if (key === "createLinearGradient") return { addColorStop: (...stop: unknown[]) => calls.push(["addColorStop", ...stop]) };
        if (key === "measureText") return { width: String(args[0]).length * 6 };
        return undefined;
      };
    },
    set(target, key: string, value) {
      target[key] = value;
      calls.push([`set:${key}`, value]);
      return true;
    },
  });
  const canvas = document.createElement("canvas");
  canvas.getContext = (() => context) as unknown as HTMLCanvasElement["getContext"];
  return { canvas, calls };
}

function tile(overrides: Partial<TreeTile> = {}): TreeTile {
  return {
    name: "item", path: "/root/item", category: "Code", bytes: 2048, files: 3, is_dir: false,
    depth: 1, has_header: false, x: 0, y: 0, width: 200, height: 100, ...overrides,
  };
}

function draw(tiles: TreeTile[], options: Partial<Parameters<typeof drawTreemap>[0]> = {}) {
  const { canvas, calls } = fakeCanvas();
  drawTreemap({
    canvas, tiles, marks: [], selectedPath: null, hoveredPath: null, mode: "size",
    width: 400, height: 300, ...options,
  });
  return { canvas, calls };
}

const named = (calls: Call[], name: string) => calls.filter((call) => call[0] === name);
const texts = (calls: Call[]) => named(calls, "fillText").map((call) => call[1]);

describe("categoryColor", () => {
  it("maps known categories and falls back to Other", () => {
    expect(categoryColor("Code")).toBe("#4f9582");
    expect(categoryColor("nope")).toBe("#4a5f5a");
  });
});

describe("drawTreemap", () => {
  it("does nothing without a 2d context or with an empty viewport", () => {
    const canvas = document.createElement("canvas");
    canvas.getContext = (() => null) as unknown as HTMLCanvasElement["getContext"];
    expect(() => drawTreemap({ canvas, tiles: [tile()], marks: [], selectedPath: null, hoveredPath: null, mode: "size", width: 10, height: 10 })).not.toThrow();
    expect(draw([tile()], { width: 0 }).calls).toEqual([]);
  });

  it("sizes the backing store and applies the zoom as a view transform", () => {
    const { canvas, calls } = draw([], { zoom: { scale: 2, x: -10, y: -20 } });
    expect([canvas.width, canvas.height]).toEqual([400, 300]);
    expect(named(calls, "translate")).toEqual([["translate", -10, -20]]);
    expect(named(calls, "scale")).toEqual([["scale", 2, 2]]);
    expect(named(draw([]).calls, "scale")).toEqual([["scale", 1, 1]]);
  });

  it("fills every tile, hatches reclaimable tiles and marks unreadable ones", () => {
    const { calls } = draw([
      tile({ reclaimable: true }),
      tile({ path: "/root/locked", unreadable: true, x: 200 }),
    ]);
    expect(named(calls, "fillRect")).toContainEqual(["fillRect", 0, 0, 200, 100]);
    expect(named(calls, "lineTo").length).toBeGreaterThan(10);
    expect(calls).toContainEqual(["set:strokeStyle", "#e5ca8755"]);
    expect(named(calls, "fillRect")).toContainEqual(["fillRect", 393, 2, 4, 4]);
  });

  it("does not hatch a reclaimable tile once it is marked and shifts its fill", () => {
    const { calls } = draw([tile({ reclaimable: true })], { marks: [{ path: "/root" }] });
    expect(named(calls, "lineTo")).toEqual([]);
    expect(calls).toContainEqual(["set:strokeStyle", "#f0d17f"]);
  });

  it("dims files that do not match the filter but never directories", () => {
    const { calls } = draw([
      tile({ name: "match.log" }),
      tile({ name: "other.bin", x: 200 }),
      tile({ name: "folder", is_dir: true, y: 100 }),
    ], { filter: "  MATCH " });
    const dims = calls.filter((call, index) => call[0] === "fillRect" && calls[index - 1]?.[1] === "#0a0c0dc4");
    expect(dims).toEqual([["fillRect", 200, 0, 200, 100]]);
  });

  it("draws a top strip on root-level tiles except in age mode", () => {
    const strip = (calls: Call[]) => named(calls, "fillRect").filter((call) => call[4] === 2);
    expect(strip(draw([tile({ depth: 0 })]).calls)).toHaveLength(1);
    expect(strip(draw([tile({ depth: 0 })], { mode: "age" }).calls)).toHaveLength(0);
  });

  it("orders outlines marked, hovered, selected and sizes them by state", () => {
    const { calls } = draw([
      tile({ path: "/s" }), tile({ path: "/h", x: 200 }), tile({ path: "/m", y: 100 }), tile({ path: null, y: 200 }),
    ], { selectedPath: "/s", hoveredPath: "/h", marks: [{ path: "/m" }] });
    const strokes = calls.filter((call) => call[0] === "strokeRect");
    expect(strokes).toEqual([
      ["strokeRect", 1, 101, 198, 98],
      ["strokeRect", 200.5, 0.5, 199, 99],
      ["strokeRect", 1, 1, 198, 98],
    ]);
    expect(calls).toContainEqual(["set:strokeStyle", "#efdcaa88"]);
    expect(calls).toContainEqual(["set:strokeStyle", "#f1d38a"]);
  });

  it("clamps outlines of zero-size tiles", () => {
    const { calls } = draw([tile({ width: 1, height: 1 })], { selectedPath: "/root/item" });
    expect(named(calls, "strokeRect")).toEqual([["strokeRect", 1, 1, 0, 0]]);
  });

  it("labels tiles with name and size, clipped to the tile", () => {
    const { calls } = draw([tile({ name: "src" })]);
    expect(named(calls, "rect")).toContainEqual(["rect", 0, 0, 200, 100]);
    expect(texts(calls)).toEqual(["src", "2.0 KB"]);
    expect(calls).toContainEqual(["set:font", "500 12px Manrope, sans-serif"]);
    expect(calls).toContainEqual(["set:fillStyle", "#f0e6cd"]);
  });

  it("uses dark ink on light fills", () => {
    const { calls } = draw([tile({ category: "Documents", depth: 0 })]);
    expect(calls).toContainEqual(["set:fillStyle", "#1b1409"]);
    expect(calls).toContainEqual(["set:fillStyle", "#3a2c14"]);
  });

  it("puts root headers in bold with a right-aligned size", () => {
    const header = { x: 0, y: 0, width: 300, height: 16 };
    const { calls } = draw([tile({ depth: 0, has_header: true, header, width: 300, height: 200, bytes: 512 })]);
    expect(calls).toContainEqual(["set:font", "700 12px Manrope, sans-serif"]);
    expect(calls).toContainEqual(["set:textAlign", "right"]);
    expect(texts(calls)).toEqual(["item", "512 B"]);
  });

  it("puts the size inline in short headers only when it fits", () => {
    const wide = draw([tile({ has_header: true, header: { x: 0, y: 0, width: 200, height: 16 } })]);
    expect(named(wide.calls, "fillText")[1]).toEqual(["fillText", "2.0 KB", 5.04 + 24 + 6.6, 3, 200 - 24 - 10.08]);
    const tight = draw([tile({ name: "a-long-name-here", has_header: true, header: { x: 0, y: 0, width: 120, height: 16 } })]);
    expect(texts(tight.calls)).toEqual(["a-long-name-here"]);
  });

  it("skips labels on tiles too small to read and values on narrow tiles", () => {
    expect(texts(draw([tile({ width: 30 })]).calls)).toEqual([]);
    expect(texts(draw([tile({ height: 10 })]).calls)).toEqual([]);
    expect(texts(draw([tile({ width: 60 })]).calls)).toEqual(["item"]);
  });

  it("labels merged and overflow tiles and file counts", () => {
    expect(texts(draw([tile({ path: null, name: "Other items (12)" })]).calls)).toEqual(["+12 more"]);
    expect(draw([tile({ path: null })]).calls).toContainEqual(["set:fillStyle", "#c9bb92"]);
    expect(texts(draw([tile({ bytes: null })]).calls)).toEqual(["item", "MERGED ITEMS"]);
    expect(texts(draw([tile({ files: 1234 })], { mode: "files" }).calls)).toEqual(["item", "1,234 files"]);
    expect(texts(draw([tile({ files: null })], { mode: "files" }).calls)).toEqual(["item", "0 files"]);
    expect(texts(draw([tile({ bytes: 5 * 1024 ** 6 })]).calls)).toEqual(["item", "5120.0 PB"]);
  });

  it("colors by age bucket in age mode", () => {
    const now = Date.now() / 1000;
    const day = 86_400;
    const base = (modified: number | null) => draw([tile({ depth: 0, modified })], { mode: "age" }).calls
      .find((call) => call[0] === "addColorStop" && call[1] === 0.45)?.[2];
    expect([1, 20, 100, 300, 900].map((days) => base(now - days * day)))
      .toEqual(["#b27943", "#99744c", "#77694f", "#5e594a", "#4b4941"]);
    // Missing timestamps fall into the oldest bucket and future timestamps into the newest.
    expect(base(null)).toBe("#4b4941");
    expect(base(now + day)).toBe("#b27943");
  });

  it("treats Windows paths under a mark as marked", () => {
    const { calls } = draw([tile({ path: "C:\\data\\file" })], { marks: [{ path: "C:\\data" }] });
    expect(calls).toContainEqual(["set:strokeStyle", "#f0d17f"]);
  });
});

describe("tileAt", () => {
  const parent = tile({ path: "/p", depth: 0, width: 100, height: 100 });
  const child = tile({ path: "/p/c", depth: 1, x: 10, y: 10, width: 20, height: 20 });

  it("returns the deepest tile under the point", () => {
    expect(tileAt([child, parent], 15, 15)?.path).toBe("/p/c");
    expect(tileAt([parent, child], 15, 15)?.path).toBe("/p/c");
    expect(tileAt([parent, child], 50, 50)?.path).toBe("/p");
  });

  it("uses half-open bounds and misses outside", () => {
    expect(tileAt([child], 30, 15)).toBeUndefined();
    expect(tileAt([parent], 100, 0)).toBeUndefined();
    expect(tileAt([parent], 0, 0)?.path).toBe("/p");
  });
});
