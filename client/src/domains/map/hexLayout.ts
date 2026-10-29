import type { HexMapData, OffsetCoords } from "./hexMap";

type Point = { x: number; y: number };

/**
 * Pixel layout of pointy-top hexes in the "odd-r" layout, like `ImageRenderer` in the `procgen` crate:
 * the top left corner of the map is at (0, 0).
 *
 * See: https://www.redblobgames.com/grids/hexagons/#hex-to-pixel
 */
export type HexLayout = {
  /** Distance from the center of a hex to its corners. */
  size: number;
  /** Width of a hex, which is also the distance between neighbors in a row. */
  width: number;
  /** Distance between rows. */
  rowHeight: number;
};

export const createHexLayout = (size: number): HexLayout => ({
  size,
  width: Math.sqrt(3) * size,
  rowHeight: 1.5 * size,
});

/** Center of the given hex. Columns outside of the map are laid out as if it repeated east-west. */
export const hexCenter = (layout: HexLayout, { col, row }: OffsetCoords): Point => ({
  x: layout.width * (col + 0.5 * (row & 1)) + layout.width / 2,
  y: layout.rowHeight * row + layout.size,
});

/** Corners of a hex centered at (0, 0), clockwise from the top one, as flat x and y pairs. */
export const hexCorners = (layout: HexLayout): number[] =>
  [-90, -30, 30, 90, 150, 210].flatMap((degrees) => {
    const angle = (Math.PI / 180) * degrees;
    return [layout.size * Math.cos(angle), layout.size * Math.sin(angle)];
  });

/** Hex containing the given point, which may lie outside of the map. */
export const hexAt = (layout: HexLayout, { x, y }: Point): OffsetCoords => {
  // relative to the center of the hex (0, 0)
  const px = x - layout.width / 2;
  const py = y - layout.size;
  const q = ((Math.sqrt(3) / 3) * px - py / 3) / layout.size;
  const r = ((2 / 3) * py) / layout.size;

  // https://www.redblobgames.com/grids/hexagons/#rounding
  const s = -q - r;
  let [roundedQ, roundedR] = [Math.round(q), Math.round(r)];
  const roundedS = Math.round(s);
  const [qDiff, rDiff, sDiff] = [Math.abs(roundedQ - q), Math.abs(roundedR - r), Math.abs(roundedS - s)];
  if (qDiff > rDiff && qDiff > sDiff) {
    roundedQ = -roundedR - roundedS;
  } else if (rDiff > sDiff) {
    roundedR = -roundedQ - roundedS;
  }
  // `>> 1` floors, including for negative rows
  return { col: roundedQ + (roundedR >> 1), row: roundedR };
};

/** Bring the given coordinates back within the map by wrapping them around east-west, or `null` beyond the poles. */
export const wrapHex = (map: HexMapData, { col, row }: OffsetCoords): OffsetCoords | null =>
  row >= 0 && row < map.rows ? { col: ((col % map.columns) + map.columns) % map.columns, row } : null;

/**
 * Horizontal bounds of a view of the map wrapping around east-west, with the map drawn twice side by side:
 * at x = 0 and at x = `period`.
 *
 * Each copy covers [0, period] on even rows and [width / 2, period + width / 2] on odd rows, so both copies
 * cover every row over [width / 2, 2 * period]: the view must start within [`minLeft`, `minLeft` + `period`)
 * and be at most `maxWidth` wide.
 */
export const wrapBounds = (layout: HexLayout, map: HexMapData) => {
  const period = layout.width * map.columns;
  return { period, minLeft: layout.width / 2, maxWidth: period - layout.width / 2 };
};

/** Height of the map, from the top corners of its first row to the bottom corners of its last one. */
export const mapHeight = (layout: HexLayout, map: HexMapData): number =>
  layout.rowHeight * (map.rows - 1) + 2 * layout.size;
