/**
 * Offset coordinates of pointy-top hexes in the "odd-r" layout, like `OffsetCoords` in the `core` crate:
 * odd rows are shoved half a hex to the east, row 0 being the north pole.
 */
export type OffsetCoords = {
  col: number;
  row: number;
};

/**
 * Rectangular map wrapping around east-west, like `RectangularHexMapShape` in the `core` crate.
 *
 * Tiles are stored row by row from north to south, each row from west to east.
 */
export type HexMapData = {
  columns: number;
  rows: number;
  /** In meters. */
  elevations: Int16Array;
  /** `TerrainKind` values. */
  terrain: Uint8Array;
};

export const tileIndex = (map: HexMapData, { col, row }: OffsetCoords): number => row * map.columns + col;
