import { TERRAIN_NAMES, TerrainKind } from "./terrain";

/** What grows on a tile over its terrain, mirroring `Vegetation` in the `core` crate. */
export const Vegetation = {
  None: 0,
  Forest: 1,
} as const;
export type Vegetation = (typeof Vegetation)[keyof typeof Vegetation];

/**
 * What the WASM bridge sends for the vegetation of a tile the viewing nation has never explored,
 * like `UNKNOWN_TERRAIN` for its terrain.
 */
export const UNKNOWN_VEGETATION = 255;

export const VEGETATION_NAMES: Record<Vegetation, string> = {
  [Vegetation.None]: "None",
  [Vegetation.Forest]: "Forest",
};

/** Whether a vegetation value, as it reaches the client, is a forest. */
export const isForest = (vegetation: number | undefined): boolean => vegetation === Vegetation.Forest;

/** What a tile is called, its forest included: "Forest", "Forested hills". */
export const groundName = (terrain: TerrainKind, forest: boolean): string => {
  if (!forest) {
    return TERRAIN_NAMES[terrain];
  }
  switch (terrain) {
    case TerrainKind.Flat:
      return "Forest";
    case TerrainKind.Hills:
      return "Forested hills";
    default:
      return `${TERRAIN_NAMES[terrain]} (forest)`;
  }
};
