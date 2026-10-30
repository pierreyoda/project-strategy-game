/** Terrain of a tile, mirroring `TerrainKind` in the `procgen` crate. */
export const TerrainKind = {
  DeepWater: 0,
  ShallowWater: 1,
  Flat: 2,
  Hills: 3,
  Mountains: 4,
  Ice: 5,
  /** Ice at the elevations of hills and mountains. */
  RoughIce: 6,
} as const;
export type TerrainKind = (typeof TerrainKind)[keyof typeof TerrainKind];

/**
 * What the WASM bridge sends for a tile the viewing nation has never explored.
 *
 * Not a terrain of the `procgen` crate: the bridge substitutes it so that the shape of the planet
 * does not travel ahead of its discovery.
 */
export const UNKNOWN_TERRAIN = 255;

/** A terrain as it reaches the client: a real one, or unknown while the tile is unexplored. */
export type TileTerrain = TerrainKind | typeof UNKNOWN_TERRAIN;

/** Whether anything is known of this tile's terrain, narrowing it to a drawable one. */
export const isExplored = (terrain: number): terrain is TerrainKind => terrain !== UNKNOWN_TERRAIN;

export const TERRAIN_NAMES: Record<TerrainKind, string> = {
  [TerrainKind.DeepWater]: "Deep water",
  [TerrainKind.ShallowWater]: "Shallow water",
  [TerrainKind.Flat]: "Flat",
  [TerrainKind.Hills]: "Hills",
  [TerrainKind.Mountains]: "Mountains",
  [TerrainKind.Ice]: "Ice",
  [TerrainKind.RoughIce]: "Rough ice",
};

/** Flat colors, for debugging and far zoomed out views. */
export const TERRAIN_COLORS: Record<TerrainKind, number> = {
  [TerrainKind.DeepWater]: 0x1f4e79,
  [TerrainKind.ShallowWater]: 0x3b7fc0,
  [TerrainKind.Flat]: 0x7da35a,
  [TerrainKind.Hills]: 0xa89060,
  [TerrainKind.Mountains]: 0x8c8478,
  [TerrainKind.Ice]: 0xe6eef2,
  [TerrainKind.RoughIce]: 0xbfd3de,
};
