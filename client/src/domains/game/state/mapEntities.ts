import { type MapEntity, SETTLEMENT_SPRITES, SETTLEMENT_TOKENS } from "../../map/entities";
import { nationColor } from "../nations";
import type { WorldState } from "./worldState";

/**
 * What the map renderer draws of a world, as the `map` domain wants it.
 *
 * The translation is all this is: the stage comes from the simulation, the color from the nation
 * table, and nothing here decides either. Fog is not applied per entity - the overlay paints over
 * the entity layer, so a settlement the viewer is no longer watching dims with the ground it
 * stands on.
 */
export const mapEntitiesOf = (world: WorldState): MapEntity[] =>
  [...world.settlements.values()].map(({ id, tile, owner, stage }) => ({
    id,
    tile,
    sprite: SETTLEMENT_SPRITES[stage],
    token: SETTLEMENT_TOKENS[stage],
    tint: nationColor(owner),
  }));
