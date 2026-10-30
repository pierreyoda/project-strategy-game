import {
  type MapEntity,
  SETTLEMENT_SPRITES,
  SETTLEMENT_TOKENS,
  buildingDrawing,
  strengthOf,
  unitDrawing,
} from "../../map/entities";
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
 * Buildings come first and units last: the simulation still raises a building on its settlement's
 * tile, a unit stands on whatever it stands on, and the draw order keeps the list's order on a
 * tile, so the settlement covers its buildings and a unit covers both (see `inDrawOrder`). The ids
 * of the three sets overlap, so a building's and a unit's are offset past any settlement's.
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
  for (const { id, tile, owner, type, health, movementLeft } of world.units.values()) {
    const drawing = unitDrawing(type);
    if (drawing) {
      entities.push({
        id: UNIT_ID_OFFSET + id,
        tile,
        ...drawing,
        tint: nationColor(owner),
        strength: strengthOf(health),
        spent: movementLeft === 0,
      });
    }
  }
  return entities;
};

/** Keep the sets' entity ids apart: each is a dense index from 0 in the simulation. */
const BUILDING_ID_OFFSET = 1 << 24;
const UNIT_ID_OFFSET = 2 << 24;
