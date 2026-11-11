/**
 * What the map draws on top of its tiles: settlements for now, units and infrastructure later.
 *
 * Nothing here knows what a nation or a turn is. An entity arrives with its drawing and its color
 * already chosen, so the renderer stays a renderer: the rules deciding which stage a settlement is
 * at, and whose it is, live in the `game` domain.
 */

import type { OffsetCoords } from "./hexMap";

/** Entity sets in `raw-assets/entities/`, each one a bundle of PixiJS's `Assets` packed in a sprite sheet by AssetPack. */
export type EntitySet = "settlements" | "buildings" | "units";

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
  /** `null` for an entity with nothing to say zoomed out, which is drawn up close only. */
  token: EntityToken | null;
  /** Color of the sprite's mark and of the token, in the owner's colors. */
  tint: number;
  /** Steps of strength to draw at the foot of a unit's counter. Left out by anything that is not a unit. */
  strength?: EntityStrength;
  /** Whether the entity has nothing left to do this turn, which dims its sprite. */
  spent?: boolean;
  /** Whether it is still being built, which fades it as the panels' portraits are. */
  ghost?: boolean;
};

/**
 * A unit's strength, like `Strength` in the `core` crate: every company it has, whatever its state,
 * then those suppressed and those lost. The rest are active.
 */
export type EntityStrength = {
  companies: number;
  suppressed: number;
  lost: number;
};

/** What a company is doing, as a counter's strength row shows it. */
export type CompanyState = "active" | "suppressed" | "lost";

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
 * What the buildings the default scenario defines are called, by the type their template names them
 * with (see `BuildingView`). Templates are script-defined: a type missing here is shown as it is.
 */
export const BUILDING_TYPE_NAMES: Record<string, string> = {
  mine: "Mine",
  water_pump: "Water pump",
  solar_plant: "Solar power plant",
};

/** The frames of a sprite key: `{key}.png`, and `{key}_mark.png` for the owner's colours if the art has one. */
const buildingSprite = (key: string): EntitySprite => ({
  entitySet: "buildings",
  base: `${key}.png`,
  mark: `${key}_mark.png`,
});

/**
 * The token's side by level, from level 1: smaller than a settlement's at every level, so that a built
 * tile never outweighs the settlement it serves. Past the last, the last.
 */
const BUILDING_TOKEN_SIDES = [0.3, 0.42, 0.55];

/**
 * The drawing of a building, or `null` for one with no sprite to draw.
 *
 * Which sprite is the template's to say, level by level (`BuildingLevelMeta.sprite`): nothing here
 * lists the buildings or the levels there are, since scripts define both. A key the sheet has no frame
 * for is left off the map by `spriteTextures`, with a warning - a building with no art yet is better
 * missing than drawn as another.
 */
export const buildingDrawing = (
  sprite: string | null,
  level: number,
): { sprite: EntitySprite; token: EntityToken } | null => {
  if (sprite === null) {
    return null;
  }
  const step = Math.min(Math.max(Math.trunc(level), 1), BUILDING_TOKEN_SIDES.length) - 1;
  return { sprite: buildingSprite(sprite), token: { side: BUILDING_TOKEN_SIDES[step] } };
};

/**
 * An empty district: a survey plot, with no token. Zoomed out, ground marked for a building is not
 * worth a shape of its own; the building that stands on it later is.
 */
export const DISTRICT_SPRITE: EntitySprite = buildingSprite("district_empty");

/**
 * The units the `units` entity set has counters for, by the type their template names them with
 * (see `UnitView`).
 */
export const UNIT_TYPES = ["infantry_division", "motorized_division", "light_tank_division"] as const;
export type UnitType = (typeof UNIT_TYPES)[number];

export const UNIT_TYPE_NAMES: Record<UnitType, string> = {
  infantry_division: "Infantry division",
  motorized_division: "Motorized division",
  light_tank_division: "Light tank division",
};

const isUnitType = (type: string): type is UnitType => (UNIT_TYPES as readonly string[]).includes(type);

const unitSprite = (type: UnitType): EntitySprite => ({
  entitySet: "units",
  base: `${type}.png`,
  mark: `${type}_mark.png`,
});

export const UNIT_SPRITES: Record<UnitType, EntitySprite> = {
  infantry_division: unitSprite("infantry_division"),
  motorized_division: unitSprite("motorized_division"),
  light_tank_division: unitSprite("light_tank_division"),
};

/** One size for every counter: a unit's weight is read from its strength row, not from its token. */
export const UNIT_TOKEN: EntityToken = { side: 0.5 };

/**
 * The circles of a counter's strength row, one per company: the active ones first, then the
 * suppressed, then the lost - so that the row reads as a bar running down from the left.
 */
export const companyStates = ({ companies, suppressed, lost }: EntityStrength): CompanyState[] => {
  const active = companies - suppressed - lost;
  return Array.from({ length: companies }, (_, company) =>
    company < active ? "active" : company < companies - lost ? "suppressed" : "lost",
  );
};

/** The most companies a counter's strength row holds in one line, before it wraps. */
export const COMPANIES_PER_ROW = 6;

/**
 * A strength row laid out in lines of `COMPANIES_PER_ROW` at most: past that, as few lines as it
 * takes, evened out, the longer first - seven companies are 4 + 3, never 6 + 1.
 */
export const companyRows = <T>(companies: readonly T[]): T[][] => {
  const rows = Math.max(Math.ceil(companies.length / COMPANIES_PER_ROW), 1);
  const perRow = Math.ceil(companies.length / rows);
  return Array.from({ length: rows }, (_, row) => companies.slice(row * perRow, (row + 1) * perRow));
};

/** How a unit's strength reads in words: "3 of 4 companies, 1 suppressed". */
export const describeStrength = ({ companies, suppressed, lost }: EntityStrength): string => {
  const remaining = `${companies - lost} of ${companies} ${companies === 1 ? "company" : "companies"}`;
  return suppressed > 0 ? `${remaining}, ${suppressed} suppressed` : remaining;
};

/**
 * The drawing of a unit, or `null` for one the set has no counter for: like `buildingDrawing`,
 * a type the simulation names and this set does not know is left off the map rather than drawn as
 * another.
 */
export const unitDrawing = (type: string | null): { sprite: EntitySprite; token: EntityToken } | null =>
  type !== null && isUnitType(type) ? { sprite: UNIT_SPRITES[type], token: UNIT_TOKEN } : null;

/** Entity sets used by the entities, to load. */
export const ENTITY_SETS: EntitySet[] = ["settlements", "buildings", "units"];

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
