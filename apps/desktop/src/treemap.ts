export type TreeRect = { x: number; y: number; width: number; height: number };

export type TreeTile = {
  name: string;
  path: string | null;
  category: string;
  bytes: number | null;
  files?: number | null;
  dirs?: number | null;
  direct_bytes?: number | null;
  parent_bytes?: number | null;
  modified?: number | null;
  is_dir: boolean;
  reclaimable?: boolean;
  unreadable?: boolean;
  depth: number;
  has_header: boolean;
  header?: TreeRect | null;
  x: number;
  y: number;
  width: number;
  height: number;
};

type DrawOptions = {
  canvas: HTMLCanvasElement;
  tiles: TreeTile[];
  marks: { path: string }[];
  selectedPath: string | null;
  hoveredPath: string | null;
  mode: "size" | "files" | "age";
  width: number;
  height: number;
  filter?: string;
  zoom?: { scale: number; x: number; y: number };
  text?: (source: string) => string;
  number?: (value: number) => string;
  decimal?: (value: number, fractionDigits?: number) => string;
};

const colors: Record<string, string> = {
  Code: "#4f9582",
  "Agent scratch": "#c4703f",
  Toolchains: "#b8923f",
  Synced: "#5b7f9c",
  Git: "#7d5f8c",
  Media: "#9a4a3f",
  Documents: "#b3a266",
  Cache: "#6d6a52",
  Other: "#4a5f5a",
};

export function categoryColor(category: string): string {
  return colors[category] ?? colors.Other;
}

export function drawTreemap(options: DrawOptions): void {
  const { canvas, tiles, marks, width, height } = options;
  const context = canvas.getContext("2d");
  if (!context || width <= 0 || height <= 0) return;

  const ratio = window.devicePixelRatio || 1;
  canvas.width = Math.round(width * ratio);
  canvas.height = Math.round(height * ratio);
  context.setTransform(ratio, 0, 0, ratio, 0, 0);
  context.clearRect(0, 0, width, height);
  // Zoom is a view transform only; tile geometry stays in base coordinates.
  const zoom = options.zoom ?? { scale: 1, x: 0, y: 0 };
  context.translate(zoom.x, zoom.y);
  context.scale(zoom.scale, zoom.scale);

  for (const tile of tiles) paintTile(context, tile, options, marks);
  paintOutlines(context, tiles, options, marks);
  paintLabels(context, tiles, options);
}

function paintTile(
  context: CanvasRenderingContext2D,
  tile: TreeTile,
  options: DrawOptions,
  marks: { path: string }[],
): void {
  const markFill = isMarked(tile, marks);
  const base = tileBase(tile, options, markFill);
  const gradient = context.createLinearGradient(tile.x, tile.y, tile.x + tile.width, tile.y + tile.height);
  gradient.addColorStop(0, mix(base, "#e4c886", 0.13));
  gradient.addColorStop(0.45, base);
  gradient.addColorStop(1, mix(base, "#090908", 0.28));
  context.fillStyle = gradient;
  context.fillRect(tile.x, tile.y, tile.width, tile.height);

  if (tile.reclaimable && !markFill) paintHatch(context, tile);
  const filter = options.filter?.trim().toLowerCase();
  if (filter && tile.path !== null && !tile.is_dir && !tile.name.toLowerCase().includes(filter)) {
    context.fillStyle = "#0a0c0dc4";
    context.fillRect(tile.x, tile.y, tile.width, tile.height);
  }
  if (tile.depth === 0 && options.mode !== "age") {
    context.fillStyle = mix(colorFor(tile, options.mode), "#e4c886", 0.32);
    context.fillRect(tile.x, tile.y, tile.width, Math.min(2, tile.height));
  }
  if (tile.unreadable && tile.width > 12 && tile.height > 12) {
    context.fillStyle = "#e4bd68";
    context.fillRect(tile.x + tile.width - 7, tile.y + 2, 4, 4);
  }
}

// Marked tiles shift toward oxblood so they read as marked, not as another category hue.
function tileBase(tile: TreeTile, options: DrawOptions, markFill: boolean): string {
  const color = colorFor(tile, options.mode);
  return markFill
    ? mix(color, "#8f3b32", 0.55)
    : tile.depth === 0
      ? color
      : mix(color, "#15140f", Math.min(0.12 + tile.depth * 0.045, 0.34));
}

function isLight(hex: string): boolean {
  const [red, green, blue] = hex.match(/[\da-f]{2}/gi)?.map((value) => Number.parseInt(value, 16)) ?? [0, 0, 0];
  return (0.299 * red + 0.587 * green + 0.114 * blue) / 255 > 0.58;
}

function paintHatch(context: CanvasRenderingContext2D, tile: TreeTile): void {
  context.save();
  context.beginPath();
  context.rect(tile.x, tile.y, tile.width, tile.height);
  context.clip();
  context.strokeStyle = "#e5ca8755";
  context.lineWidth = 1;
  for (let x = tile.x - tile.height; x < tile.x + tile.width; x += 7) {
    context.beginPath();
    context.moveTo(x, tile.y + tile.height);
    context.lineTo(x + tile.height, tile.y);
    context.stroke();
  }
  context.restore();
}

function paintOutlines(
  context: CanvasRenderingContext2D,
  tiles: TreeTile[],
  options: DrawOptions,
  marks: { path: string }[],
): void {
  const rings = tiles.flatMap((tile) => {
    const marked = isMarked(tile, marks);
    const selected = tile.path !== null && tile.path === options.selectedPath;
    const hovered = tile.path !== null && tile.path === options.hoveredPath;
    if (!marked && !selected && !hovered) return [];
    return [{ tile, marked, selected, hovered, rank: selected ? 3 : hovered ? 2 : 1 }];
  }).sort((left, right) => left.rank - right.rank);
  for (const { tile, marked, selected, hovered } of rings) {
    context.lineWidth = selected ? 2 : hovered ? 1 : 2;
    context.strokeStyle = selected ? "#f1d38a" : marked ? "#f0d17f" : "#efdcaa88";
    context.strokeRect(
      tile.x + context.lineWidth / 2,
      tile.y + context.lineWidth / 2,
      Math.max(0, tile.width - context.lineWidth),
      Math.max(0, tile.height - context.lineWidth),
    );
  }
}

function paintLabels(
  context: CanvasRenderingContext2D,
  tiles: TreeTile[],
  options: DrawOptions,
): void {
  const sorted = [...tiles]
    .sort((left, right) => right.width * right.height - left.width * left.height)
    .slice(0, 150);
  for (const tile of sorted) {
    const light = isLight(tileBase(tile, options, isMarked(tile, options.marks)));
    paintLabel(context, tile, options.mode, light, options);
  }
}

function paintLabel(
  context: CanvasRenderingContext2D,
  tile: TreeTile,
  mode: "size" | "files" | "age",
  lightFill: boolean,
  options: DrawOptions,
): void {
  const region = tile.header ?? tile;
  const nameSize = 12;
  const sizeSize = 11;
  if (region.width < nameSize * 3.3 || region.height < nameSize) return;

  const pad = nameSize * 0.42;
  const inset = nameSize * 0.25;
  const otherItemCount = tile.name.match(/^Other items \((\d+)\)$/)?.[1];
  const text = options.text ?? ((source: string) => source);
  const number = options.number ?? String;
  const name = tile.path === null
    ? otherItemCount ? `+${number(Number(otherItemCount))} ${text("more")}` : tile.name
    : tile.name;
  const value = tile.path === null
    ? ""
    : tile.bytes === null
      ? text("MERGED ITEMS")
      : mode === "files"
        ? `${number(tile.files ?? 0)} ${text("files")}`
        : formatBytes(tile.bytes, options.decimal);
  context.save();
  context.beginPath();
  context.rect(region.x, region.y, region.width, region.height);
  context.clip();
  context.textBaseline = "top";
  context.textAlign = "left";
  context.fillStyle = lightFill ? "#1b1409" : tile.path === null ? "#c9bb92" : "#f0e6cd";
  context.font = `${tile.depth === 0 && tile.has_header ? 700 : 500} ${nameSize}px Manrope, sans-serif`;
  context.fillText(name, region.x + pad, region.y + inset, Math.max(0, region.width - pad * 2));

  if (value && region.width > nameSize * 6) {
    context.font = `400 ${sizeSize}px "Spline Sans Mono", monospace`;
    context.fillStyle = lightFill ? "#3a2c14" : "#c4b48c";
    if (tile.has_header && tile.depth === 0) {
      context.textAlign = "right";
      context.fillText(value, region.x + region.width - pad, region.y + inset, region.width * 0.45);
    } else if (!tile.has_header && region.height >= nameSize * 2.3) {
      context.textAlign = "left";
      context.fillText(value, region.x + pad, region.y + inset + nameSize * 1.4, region.width - pad * 2);
    } else {
      context.textAlign = "left";
      const textWidth = context.measureText(name).width;
      if (region.width - textWidth - pad * 2 > sizeSize * 3) {
        context.fillText(value, region.x + pad + textWidth + sizeSize * 0.6, region.y + inset, region.width - textWidth - pad * 2);
      }
    }
  }
  context.restore();
}

export function tileAt(
  tiles: TreeTile[],
  x: number,
  y: number,
): TreeTile | undefined {
  return [...tiles]
    .filter((tile) => x >= tile.x && x < tile.x + tile.width && y >= tile.y && y < tile.y + tile.height)
    .sort((left, right) => right.depth - left.depth)[0];
}

function colorFor(tile: TreeTile, mode: "size" | "files" | "age"): string {
  if (mode !== "age") return colors[tile.category] ?? colors.Other;
  const days = tile.modified
    ? Math.max(0, Date.now() / 1000 - tile.modified) / 86_400
    : Number.POSITIVE_INFINITY;
  return days <= 7 ? "#b27943" : days <= 30 ? "#99744c"
    : days <= 182 ? "#77694f" : days <= 365 ? "#5e594a" : "#4b4941";
}

function isMarked(tile: TreeTile, marks: { path: string }[]): boolean {
  const path = tile.path;
  if (path === null) return false;
  const separator = path.includes("\\") ? "\\" : "/";
  return marks.some((mark) => path === mark.path || path.startsWith(`${mark.path}${separator}`));
}

function mix(first: string, second: string, amount: number): string {
  const left = first.match(/[\da-f]{2}/gi)?.map((value) => Number.parseInt(value, 16)) ?? [0, 0, 0];
  const right = second.match(/[\da-f]{2}/gi)?.map((value) => Number.parseInt(value, 16)) ?? [0, 0, 0];
  return `#${left.map((value, index) => Math.round(value + (right[index] - value) * amount).toString(16).padStart(2, "0")).join("")}`;
}

function formatBytes(value: number, decimal?: (value: number, fractionDigits?: number) => string): string {
  if (value < 1024) return `${value} B`;
  const units = ["KB", "MB", "GB", "TB", "PB"];
  let amount = value;
  let index = -1;
  do { amount /= 1024; index += 1; } while (amount >= 1024 && index < units.length - 1);
  return `${decimal ? decimal(amount) : amount.toFixed(1)} ${units[index]}`;
}
