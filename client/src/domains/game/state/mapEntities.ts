import type { MapConstruction } from "../../map/constructions";
import {
  DISTRICT_SPRITE,
  type MapEntity,
  SETTLEMENT_SPRITES,
  SETTLEMENT_TOKENS,
  buildingDrawing,
  unitDrawing,
} from "../../map/entities";
import { nationColor } from "../nations";
import { BuildingState, TileFlags } from "../session/protocol";
import type { WorldState } from "./worldState";

/**
 * What the map renderer draws of a world, as the `map` domain wants it.
 *
 * The translation is all this is: the stage and the level come from the simulation, the color from
 * the nation table, and nothing here decides either. Fog is not applied per entity - the overlay
 * paints over the entity layer, so a settlement the viewer is no longer watching dims with the
 * ground it stands on.
 *
 * Districts come first, buildings next and units last: a district is ground marked out, the
 * simulation can still raise an extractor on its settlement's tile, a unit stands on whatever it
 * stands on, and the draw order keeps the list's order on a tile, so the settlement covers its
 * buildings and a unit covers all of them (see `inDrawOrder`). The ids of the four sets overlap, so
 * a district's, a building's and a unit's are offset past any settlement's.
 */
export const mapEntitiesOf = (world: WorldState): MapEntity[] => {
  const entities: MapEntity[] = [];
  world.tileFlags.forEach((flags, tile) => {
    // an empty district only: a building on it draws its own ground works
    if ((flags & TileFlags.District) === 0 || (flags & (TileFlags.Building | TileFlags.Settlement)) !== 0) {
      return;
    }
    entities.push({
      id: DISTRICT_ID_OFFSET + tile,
      tile,
      sprite: DISTRICT_SPRITE,
      token: null,
      tint: nationColor(world.tileOwner[tile]),
      // still being laid out: faded, like a building going up
      ghost: (flags & TileFlags.DistrictWorks) !== 0,
    });
  });
  for (const { id, tile, owner, sprite, level, state } of world.buildings.values()) {
    const drawing = buildingDrawing(sprite, level);
    if (drawing) {
      entities.push({
        id: BUILDING_ID_OFFSET + id,
        tile,
        ...drawing,
        tint: nationColor(owner),
        // still going up: faded, a rival's as much as the viewer's - scaffolding shows from afar
        ghost: (state & BuildingState.UnderConstruction) !== 0,
      });
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
  for (const { id, tile, owner, type, strength, movementLeft } of world.units.values()) {
    const drawing = unitDrawing(type);
    if (drawing) {
      entities.push({
        id: UNIT_ID_OFFSET + id,
        tile,
        ...drawing,
        tint: nationColor(owner),
        strength,
        spent: movementLeft === 0,
      });
    }
  }
  return entities;
};

/**
 * The works the construction overlay draws the progress of: the viewer's own constructions and
 * upgrades. A rival's are sent with no turns - only that they are under way, which `mapEntitiesOf`
 * shows - so there is no progress of theirs to draw.
 */
export const constructionsOf = (world: WorldState): MapConstruction[] =>
  [...world.buildings.values()]
    .filter(({ turnsTotal }) => turnsTotal > 0)
    .map(({ id, tile, turnsLeft, turnsTotal }) => ({ id, tile, turnsLeft, turnsTotal }));

/** Keep the sets' entity ids apart: each is a dense index from 0 in the simulation, a tile's for a district. */
const BUILDING_ID_OFFSET = 1 << 24;
const UNIT_ID_OFFSET = 2 << 24;
const DISTRICT_ID_OFFSET = 3 << 24;
