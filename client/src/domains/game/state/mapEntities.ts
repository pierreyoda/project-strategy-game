import { type MapEntity, SETTLEMENT_SPRITES, SETTLEMENT_TOKENS, buildingDrawing } from "../../map/entities";
import { nationColor } from "../nations";
import type { WorldState } from "./worldState";

/**
 * What the map renderer draws of a world, as the `map` domain wants it.
 *
 * The translation is all this is: the stage and the level come from the simulation, the color from
 * the nation table, and nothing here decides either. Fog is not applied per entity - the overlay
 * paints over the entity layer, so a settlement the viewer is no longer watching dims with the
 * ground it stands on.
 *
 * Buildings come first: the simulation still raises them on their settlement's tile, and the draw
 * order keeps the list's order on a tile, so the settlement ends up on top (see `inDrawOrder`).
 * The ids of the two overlap, so a building's is offset past any settlement's.
 */
export const mapEntitiesOf = (world: WorldState): MapEntity[] => {
  const entities: MapEntity[] = [];
  for (const { id, tile, owner, type, level } of world.buildings.values()) {
    const drawing = buildingDrawing(type, level);
    if (drawing) {
      entities.push({ id: BUILDING_ID_OFFSET + id, tile, ...drawing, tint: nationColor(owner) });
    }
  }
  for (const { id, tile, owner, stage } of world.settlements.values()) {
    entities.push({
      id,
      tile,
      sprite: SETTLEMENT_SPRITES[stage],
      token: SETTLEMENT_TOKENS[stage],
      tint: nationColor(owner),
    });
  }
  return entities;
};

/** Keeps a building's entity id apart from a settlement's: both are dense indices from 0 in the simulation. */
const BUILDING_ID_OFFSET = 1 << 24;
