import type { OffsetCoords } from "./hexMap";
import type { MapLook } from "./mapLook";
import { TerrainKind } from "./terrain";

/**
 * The bundle of PixiJS's `Assets` holding every tile still drawn, feathered and packed in one sprite sheet
 * by AssetPack from `raw-assets/features{m}{tps}/`, which `scripts/feather-tiles.ts` fills from the tile
 * sets of `raw-assets/tiles/`. The frames keep the names they had there.
 */
export const FEATURES_BUNDLE = "features";

/**
 * Tiles of a terrain, as frame names in the features bundle.
 *
 * The `decorated` tiles show some details (rocks, stones...): they are picked less often than the `base` ones.
 * A terrain without a `feature` has no base tiles left to draw: the ground layer paints its ground.
 */
export type TerrainTiles = {
  base: string[];
  decorated: string[];
  /**
   * Lit from no particular side, so a mirrored copy tells no lie: flips come for free as variants.
   * Rocks and ridges are lit from the upper left, and a flipped one next to its neighbors shows two suns.
   */
  mirrorable: boolean;
  /**
   * Whether the base tiles paint something standing on the ground - rocks, peaks, ridges - rather than
   * the ground alone. The ground is blended by the ground layer now, so a tile that is only ground
   * has nothing left to draw; one with a feature on it still does, until the features are decals.
   */
  feature: boolean;
};

/** Whether the given tile draws one of its terrain's decorated tiles, see `pickTileVariant`. */
export const pickTileIsDecorated = (
  tiles: { decorated: readonly unknown[] },
  coords: OffsetCoords,
  { decoratedShare }: Pick<MapLook, "decoratedShare">,
): boolean => tiles.decorated.length > 0 && tileRandom(coords, 1) < decoratedShare;

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
export const pickTileVariant = <T>(
  tiles: { base: readonly T[]; decorated: readonly T[] },
  coords: OffsetCoords,
  { decoratedShare }: Pick<MapLook, "decoratedShare">,
): T => {
  const variants = pickTileIsDecorated(tiles, coords, { decoratedShare }) ? tiles.decorated : tiles.base;
  return variants[Math.floor(tileRandom(coords, 2) * variants.length)];
};

/** Whether to mirror the tile drawn on the given tile along each axis, only ever for a `mirrorable` terrain. */
export const pickTileFlip = (terrain: TerrainKind, coords: OffsetCoords): { x: boolean; y: boolean } =>
  TERRAIN_TILES[terrain].mirrorable
    ? { x: tileRandom(coords, 3) < 0.5, y: tileRandom(coords, 4) < 0.5 }
    : { x: false, y: false };

/** The tint of the given tile, a few percent under white, see `MapLook`. */
export const pickTileTint = (
  coords: OffsetCoords,
  { tintBrightness, tintHue }: Pick<MapLook, "tintBrightness" | "tintHue">,
): number => {
  const brightness = 1 - tintBrightness * tileRandom(coords, 5);
  const channel = (salt: number) => Math.round(255 * (brightness - tintHue * tileRandom(coords, salt)));
  return (channel(6) << 16) | (channel(7) << 8) | channel(8);
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
    feature: false,
    mirrorable: true,
    // ground only, see `feature`: the ground layer paints it
    base: [],
    decorated: [],
  },
  [TerrainKind.ShallowWater]: {
    feature: false,
    mirrorable: true,
    base: [],
    // scattered rocks
    decorated: ["water_tile_02.png"],
  },
  [TerrainKind.Flat]: {
    feature: false,
    mirrorable: true,
    base: [],
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
    feature: true,
    mirrorable: false,
    // rocky outcrops, boulders, a grassy mound
    base: ["nature_tile_06.png", "nature_tile_13.png", "nature_tile_16.png", "nature_tile_21.png"],
    decorated: [],
  },
  [TerrainKind.Mountains]: {
    feature: true,
    mirrorable: false,
    base: ["nature_tile_04.png", "nature_tile_05.png", "nature_tile_14.png"],
    decorated: [],
  },
  [TerrainKind.Ice]: {
    feature: false,
    mirrorable: true,
    base: [],
    decorated: [],
  },
  [TerrainKind.RoughIce]: {
    feature: true,
    mirrorable: false,
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
