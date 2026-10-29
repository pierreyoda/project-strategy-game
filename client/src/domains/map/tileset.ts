import type { OffsetCoords } from "./hexMap";
import { TerrainKind } from "./terrain";

/** Tile sets in `raw-assets/tiles/`, each one a bundle of PixiJS's `Assets` packed in a sprite sheet by AssetPack. */
export type TileSet = "autumn" | "desert" | "ice" | "lava" | "nature" | "water";

/**
 * Tiles of a terrain, as frame names in the sprite sheet of their tile set.
 *
 * The `decorated` tiles show some details (rocks, stones...): they are picked less often than the `base` ones.
 */
export type TerrainTiles = {
  tileSet: TileSet;
  base: string[];
  decorated: string[];
};

/** Share of the tiles of a terrain picked among its decorated tiles, when it has some. */
export const DECORATED_TILES_SHARE = 0.25;

/** Pseudo-random number in [0, 1), always the same for the given tile and salt (hashed with "lowbias32"). */
const tileRandom = ({ col, row }: OffsetCoords, salt: number): number => {
  let hash = Math.imul(col, 0x27d4eb2d) ^ Math.imul(row, 0x165667b1) ^ salt;
  hash = Math.imul(hash ^ (hash >>> 16), 0x7feb352d);
  hash = Math.imul(hash ^ (hash >>> 15), 0x846ca68b);
  return ((hash ^ (hash >>> 16)) >>> 0) / 2 ** 32;
};

/**
 * Variant of a terrain's tiles to draw on the given tile: always the same one, so that the map looks the same
 * wherever and whenever it is drawn.
 */
export const pickTileVariant = <T>(tiles: { base: readonly T[]; decorated: readonly T[] }, coords: OffsetCoords): T => {
  const isDecorated = tiles.decorated.length > 0 && tileRandom(coords, 1) < DECORATED_TILES_SHARE;
  const variants = isDecorated ? tiles.decorated : tiles.base;
  return variants[Math.floor(tileRandom(coords, 2) * variants.length)];
};

/*
 * Unused tiles:
 * - vegetation, until the map generator places some: nature 02 (forest), 07 and 12 (shrubs), 18 and 19 (forested peaks);
 * - features, until the map generator places some: water 01 (island), 04 (rocky islet), 06 (rock formation),
 *   07 (sea cave); ice 06 (ice block), 09 (glacier plateau), 10 (ice hole), 14 (ice cave), 17 (frozen ponds);
 *   nature 09 (crystals), 15 (cave), 17 (crater);
 * - climates, until the map generator has some: the autumn, desert and lava tile sets.
 */
export const TERRAIN_TILES: Record<TerrainKind, TerrainTiles> = {
  [TerrainKind.DeepWater]: {
    tileSet: "water",
    base: ["water_tile_05.png"],
    decorated: [],
  },
  [TerrainKind.ShallowWater]: {
    tileSet: "water",
    base: ["water_tile_03.png"],
    // scattered rocks
    decorated: ["water_tile_02.png"],
  },
  [TerrainKind.Flat]: {
    tileSet: "nature",
    base: ["nature_tile_01.png", "nature_tile_20.png"],
    // stone slabs, scattered boulders, sand, a large flat rock
    decorated: [
      "nature_tile_03.png",
      "nature_tile_08.png",
      "nature_tile_10.png",
      "nature_tile_11.png",
      "nature_tile_22.png",
    ],
  },
  [TerrainKind.Hills]: {
    tileSet: "nature",
    // rocky outcrops, boulders, a grassy mound
    base: ["nature_tile_06.png", "nature_tile_13.png", "nature_tile_16.png", "nature_tile_21.png"],
    decorated: [],
  },
  [TerrainKind.Mountains]: {
    tileSet: "nature",
    base: ["nature_tile_04.png", "nature_tile_05.png", "nature_tile_14.png"],
    decorated: [],
  },
  [TerrainKind.Ice]: {
    tileSet: "ice",
    base: [
      "ice_tile_01.png",
      "ice_tile_02.png",
      "ice_tile_03.png",
      "ice_tile_04.png",
      "ice_tile_12.png",
      "ice_tile_15.png",
      "ice_tile_18.png",
      "ice_tile_19.png",
    ],
    decorated: [],
  },
  [TerrainKind.RoughIce]: {
    tileSet: "ice",
    // ridges, mounds, cliffs
    base: [
      "ice_tile_05.png",
      "ice_tile_07.png",
      "ice_tile_08.png",
      "ice_tile_11.png",
      "ice_tile_13.png",
      "ice_tile_16.png",
    ],
    decorated: [],
  },
};

/** Tile sets used by the terrains, to load. */
export const TERRAIN_TILE_SETS: TileSet[] = [...new Set(Object.values(TERRAIN_TILES).map(({ tileSet }) => tileSet))];
