/**
 * What the map draws on top of its tiles: settlements for now, units and infrastructure later.
 *
 * Nothing here knows what a nation or a turn is. An entity arrives with its drawing and its color
 * already chosen, so the renderer stays a renderer: the rules deciding which stage a settlement is
 * at, and whose it is, live in the `game` domain.
 */

import type { OffsetCoords } from "./hexMap";

/** Entity sets in `raw-assets/entities/`, each one a bundle of PixiJS's `Assets` packed in a sprite sheet by AssetPack. */
export type EntitySet = "settlements";

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

/** Entity sets used by the entities, to load. */
export const ENTITY_SETS: EntitySet[] = ["settlements"];

/**
 * The entities in the order they have to be drawn: by row from north to south, then west to east.
 *
 * Settlements overhang the top of their hex, so a southern neighbor has to be drawn after - and over -
 * the one above it, the way the tiles themselves are laid down. Sorting a copy leaves the caller's
 * list alone, which is state the `game` domain owns.
 */
export const inDrawOrder = (entities: readonly MapEntity[]): MapEntity[] =>
  [...entities].sort((first, second) => first.tile - second.tile);

/** The coordinates of a row-major tile index, on a map of the given width. */
export const entityCoords = (columns: number, tile: number): OffsetCoords => ({
  col: tile % columns,
  row: Math.floor(tile / columns),
});
