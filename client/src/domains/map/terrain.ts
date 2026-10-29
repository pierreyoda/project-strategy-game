/** Terrain of a tile. */
export const TerrainKind = {
  DeepWater: 0,
  ShallowWater: 1,
  Flat: 2,
  Hills: 3,
  Mountains: 4,
  Ice: 5,
} as const;
export type TerrainKind = (typeof TerrainKind)[keyof typeof TerrainKind];

export const TERRAIN_NAMES: Record<TerrainKind, string> = {
  [TerrainKind.DeepWater]: "Deep water",
  [TerrainKind.ShallowWater]: "Shallow water",
  [TerrainKind.Flat]: "Flat",
  [TerrainKind.Hills]: "Hills",
  [TerrainKind.Mountains]: "Mountains",
  [TerrainKind.Ice]: "Ice",
};

/** Flat colors, for debugging and far zoomed out views. */
export const TERRAIN_COLORS: Record<TerrainKind, number> = {
  [TerrainKind.DeepWater]: 0x1f4e79,
  [TerrainKind.ShallowWater]: 0x3b7fc0,
  [TerrainKind.Flat]: 0x7da35a,
  [TerrainKind.Hills]: 0xa89060,
  [TerrainKind.Mountains]: 0x8c8478,
  [TerrainKind.Ice]: 0xe6eef2,
};
