/**
 * What the map draws on top of its tiles: settlements for now, units and infrastructure later.
 *
 * Nothing here knows what a nation or a turn is. An entity arrives with its drawing and its color
 * already chosen, so the renderer stays a renderer: the rules deciding which stage a settlement is
 * at, and whose it is, live in the `game` domain.
 */

import type { OffsetCoords } from "./hexMap";

/** Entity sets in `raw-assets/entities/`, each one a bundle of PixiJS's `Assets` packed in a sprite sheet by AssetPack. */
export type EntitySet = "settlements" | "buildings";

/**
 * The drawing of an entity, as frame names in the sprite sheet of its entity set.
 *
 * The art is split in two: `base` holds everything painted in the shared palette of the tile sets,
 * `mark` the few surfaces that take the owner's color - banners, hull stripes, the dome's ring. The
 * mark is drawn white so that tinting it gives the color itself rather than a muddied blend, and it
 * is a separate frame rather than a recolored copy of the whole sprite: one set of art serves every
 * nation.
 */
export type EntitySprite = {
  entitySet: EntitySet;
  base: string;
  /** Left out by an entity that is not painted in anyone's colors. */
  mark?: string;
};

/**
 * The shape standing in for a sprite while hexes are too small on screen to read one.
 *
 * Zoomed out, a settlement has to answer "whose, and how big" in a few pixels; its silhouette cannot.
 * The token is drawn in the owner's color at a size that grows with the stage, which is exactly those
 * two questions and nothing else.
 */
export type EntityToken = {
  /** Side of the token, as a share of the distance from a hex's center to its corners. */
  side: number;
};

/**
 * An entity to draw, placed on a tile.
 *
 * Pure data, rebuilt from the world's state rather than mutated: there are few enough of these that
 * handing the renderer a new list costs less than tracking which ones moved.
 */
export type MapEntity = {
  id: number;
  /** Row-major tile index. */
  tile: number;
  sprite: EntitySprite;
  token: EntityToken;
  /** Color of the sprite's mark and of the token, in the owner's colors. */
  tint: number;
};

/** How far a settlement has grown, mirroring `SettlementStage` in the `core` crate. */
export const SettlementStage = {
  Outpost: 0,
  Colony: 1,
  Town: 2,
  City: 3,
  Capital: 4,
} as const;
export type SettlementStage = (typeof SettlementStage)[keyof typeof SettlementStage];

export const SETTLEMENT_STAGE_NAMES: Record<SettlementStage, string> = {
  [SettlementStage.Outpost]: "Outpost",
  [SettlementStage.Colony]: "Colony",
  [SettlementStage.Town]: "Town",
  [SettlementStage.City]: "City",
  [SettlementStage.Capital]: "Capital",
};

const settlementSprite = (stage: string): EntitySprite => ({
  entitySet: "settlements",
  base: `settlement_${stage}.png`,
  mark: `settlement_${stage}_mark.png`,
});

export const SETTLEMENT_SPRITES: Record<SettlementStage, EntitySprite> = {
  [SettlementStage.Outpost]: settlementSprite("outpost"),
  [SettlementStage.Colony]: settlementSprite("colony"),
  [SettlementStage.Town]: settlementSprite("town"),
  [SettlementStage.City]: settlementSprite("city"),
  [SettlementStage.Capital]: settlementSprite("capital"),
};

/** Tracking the footprints the sprites themselves grow through, from a corner of the hex to most of it. */
export const SETTLEMENT_TOKENS: Record<SettlementStage, EntityToken> = {
  [SettlementStage.Outpost]: { side: 0.4 },
  [SettlementStage.Colony]: { side: 0.55 },
  [SettlementStage.Town]: { side: 0.7 },
  [SettlementStage.City]: { side: 0.85 },
  [SettlementStage.Capital]: { side: 1 },
};

/**
 * The buildings the `buildings` entity set has sprites for, by the type their template names them
 * with (see `BuildingView`), and the levels each one is drawn at.
 */
export const BUILDING_TYPES = ["mine", "solar_plant"] as const;
export type BuildingType = (typeof BUILDING_TYPES)[number];
export const BUILDING_LEVELS = [1, 2, 3] as const;
export type BuildingLevel = (typeof BUILDING_LEVELS)[number];

export const BUILDING_TYPE_NAMES: Record<BuildingType, string> = {
  mine: "Mine",
  solar_plant: "Solar power plant",
};

const isBuildingType = (type: string): type is BuildingType => (BUILDING_TYPES as readonly string[]).includes(type);

const buildingSprite = (type: BuildingType, level: BuildingLevel): EntitySprite => ({
  entitySet: "buildings",
  base: `${type}_${level}.png`,
  mark: `${type}_${level}_mark.png`,
});

export const BUILDING_SPRITES: Record<BuildingType, Record<BuildingLevel, EntitySprite>> = {
  mine: { 1: buildingSprite("mine", 1), 2: buildingSprite("mine", 2), 3: buildingSprite("mine", 3) },
  solar_plant: {
    1: buildingSprite("solar_plant", 1),
    2: buildingSprite("solar_plant", 2),
    3: buildingSprite("solar_plant", 3),
  },
};

/** Smaller than a settlement's at every level: a built tile never outweighs the settlement it serves. */
export const BUILDING_TOKENS: Record<BuildingLevel, EntityToken> = {
  1: { side: 0.3 },
  2: { side: 0.42 },
  3: { side: 0.55 },
};

/**
 * The drawing of a building, or `null` for one the set has no sprite for.
 *
 * A type the simulation names and this set does not know is not an error here: templates are
 * script-defined, and a building with no art yet is better left off the map than drawn as another.
 * A level past the last one drawn keeps the last sprite, so an upgrade never undraws anything.
 */
export const buildingDrawing = (
  type: string | null,
  level: number,
): { sprite: EntitySprite; token: EntityToken } | null => {
  if (type === null || !isBuildingType(type)) {
    return null;
  }
  const drawn = Math.min(Math.max(Math.trunc(level), BUILDING_LEVELS[0]), BUILDING_LEVELS.at(-1)!) as BuildingLevel;
  return { sprite: BUILDING_SPRITES[type][drawn], token: BUILDING_TOKENS[drawn] };
};

/** Entity sets used by the entities, to load. */
export const ENTITY_SETS: EntitySet[] = ["settlements", "buildings"];

/**
 * The entities in the order they have to be drawn: by row from north to south, then west to east.
 *
 * Settlements overhang the top of their hex, so a southern neighbor has to be drawn after - and over -
 * the one above it, the way the tiles themselves are laid down. The sort is stable, so two entities
 * on one tile keep the caller's order: a settlement listed after the buildings of its tile is drawn
 * over them. Sorting a copy leaves the caller's list alone, which is state the `game` domain owns.
 */
export const inDrawOrder = (entities: readonly MapEntity[]): MapEntity[] =>
  [...entities].sort((first, second) => first.tile - second.tile);

/** The coordinates of a row-major tile index, on a map of the given width. */
export const entityCoords = (columns: number, tile: number): OffsetCoords => ({
  col: tile % columns,
  row: Math.floor(tile / columns),
});
