import { type MapEntity, SETTLEMENT_SPRITES, SETTLEMENT_TOKENS, SettlementStage } from "./entities";
import type { HexMapData } from "./hexMap";
import { TerrainKind } from "./terrain";

/**
 * Deterministic stand-in for a session's settlements, to work on the entity rendering without a
 * running simulation, the way `createFakeHexMap` stands in for a generated planet.
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

/** Scatter settlements of every stage over the habitable tiles of a map, `density` of them taking one. */
export const createFakeEntities = (map: HexMapData, density = 0.05): MapEntity[] => {
  const entities: MapEntity[] = [];
  for (let tile = 0; tile < map.rows * map.columns; tile++) {
    if (!SETTLEABLE.includes(map.terrain[tile]) || entityRandom(tile, 1) >= density) {
      continue;
    }
    const stage = STAGES[Math.floor(entityRandom(tile, 2) * STAGES.length)];
    entities.push({
      id: tile,
      tile,
      sprite: SETTLEMENT_SPRITES[stage],
      token: SETTLEMENT_TOKENS[stage],
      tint: FAKE_TINTS[Math.floor(entityRandom(tile, 3) * FAKE_TINTS.length)],
    });
  }
  return entities;
};
