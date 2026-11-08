import {
  type MapEntity,
  SETTLEMENT_SPRITES,
  SETTLEMENT_TOKENS,
  SettlementStage,
  UNIT_TYPES,
  buildingDrawing,
  unitDrawing,
} from "./entities";
import type { MapConstruction } from "./constructions";
import type { HexMapData } from "./hexMap";
import { TerrainKind } from "./terrain";

/**
 * Deterministic stand-in for a session's settlements and buildings, to work on the entity rendering
 * without a running simulation, the way `createFakeHexMap` stands in for a generated planet.
 *
 * The colors are its own rather than the `game` domain's: the map draws what it is handed, and a
 * test bench of the map has no nations to ask.
 */
const FAKE_TINTS = [0x4f9ddb, 0xd4604f, 0x6fbf73, 0xd9a441];

/** Terrains a settlement is placed on: enough of them to tell the sprites apart from the ground. */
const SETTLEABLE: readonly number[] = [TerrainKind.Flat, TerrainKind.Hills];

const STAGES = Object.values(SettlementStage);

/** Pseudo-random number in [0, 1), always the same for the given tile and salt (hashed with "lowbias32"). */
const entityRandom = (tile: number, salt: number): number => {
  let hash = Math.imul(tile, 0x27d4eb2d) ^ Math.imul(salt, 0x165667b1);
  hash = Math.imul(hash ^ (hash >>> 16), 0x7feb352d);
  hash = Math.imul(hash ^ (hash >>> 15), 0x846ca68b);
  return ((hash ^ (hash >>> 16)) >>> 0) / 2 ** 32;
};

/**
 * The buildings the `buildings` sheet has art for today, and how many levels each is drawn at: what
 * the test bench scatters around. The game itself draws whatever its scripts name (see
 * `buildingDrawing`); the fakes only follow the naming scripts default to, `{type}_{level}`.
 */
const FAKE_BUILDINGS = ["mine", "solar_plant"] as const;
const FAKE_LEVELS = 3;

/**
 * Scatter settlements of every stage over the habitable tiles of a map, `density` of them taking one,
 * as many buildings of every type and level on their own tiles, and half as many units, at every
 * strength, some of them spent.
 */
export const createFakeEntities = (map: HexMapData, density = 0.05): MapEntity[] => {
  const entities: MapEntity[] = [];
  for (let tile = 0; tile < map.rows * map.columns; tile++) {
    const roll = entityRandom(tile, 1);
    if (!SETTLEABLE.includes(map.terrain[tile]) || roll >= 2.5 * density) {
      continue;
    }
    const tint = FAKE_TINTS[Math.floor(entityRandom(tile, 3) * FAKE_TINTS.length)];
    if (roll < density) {
      const stage = STAGES[Math.floor(entityRandom(tile, 2) * STAGES.length)];
      entities.push({ id: tile, tile, sprite: SETTLEMENT_SPRITES[stage], token: SETTLEMENT_TOKENS[stage], tint });
    } else if (roll < 2 * density) {
      const type = FAKE_BUILDINGS[Math.floor(entityRandom(tile, 4) * FAKE_BUILDINGS.length)];
      const level = 1 + Math.floor(entityRandom(tile, 5) * FAKE_LEVELS);
      entities.push({ id: tile, tile, ...buildingDrawing(`${type}_${level}`, level)!, tint });
    } else {
      const type = UNIT_TYPES[Math.floor(entityRandom(tile, 6) * UNIT_TYPES.length)];
      // 3 or 4 companies, some of them suppressed or lost
      const companies = 3 + Math.floor(entityRandom(tile, 7) * 2);
      const lost = Math.floor(entityRandom(tile, 9) * companies);
      const suppressed = Math.floor(entityRandom(tile, 10) * (companies - lost));
      entities.push({
        id: tile,
        tile,
        ...unitDrawing(type)!,
        tint,
        strength: { companies, suppressed, lost },
        spent: entityRandom(tile, 8) < 0.3,
      });
    }
  }
  return entities;
};

/**
 * Constructions on some of the tiles `createFakeEntities` puts a building on, at the same `density`:
 * turned on together, the overlays sit over the buildings they are raising.
 */
export const createFakeConstructions = (map: HexMapData, density = 0.05): MapConstruction[] => {
  const constructions: MapConstruction[] = [];
  for (let tile = 0; tile < map.rows * map.columns; tile++) {
    const roll = entityRandom(tile, 1);
    // the buildings' band of `createFakeEntities`, and two in five of them
    if (!SETTLEABLE.includes(map.terrain[tile]) || roll < density || roll >= 2 * density) {
      continue;
    }
    if (entityRandom(tile, 8) >= 0.4) {
      continue;
    }
    const turnsTotal = 2 + Math.floor(entityRandom(tile, 9) * 11);
    const turnsLeft = 1 + Math.floor(entityRandom(tile, 10) * turnsTotal);
    constructions.push({ id: tile, tile, turnsLeft, turnsTotal });
  }
  return constructions;
};
