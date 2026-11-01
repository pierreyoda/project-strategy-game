import { type DepositLevel, type MapDeposit, isDepositResource } from "../../map/deposits";
import { DEPOSIT_LEVELS } from "../session/protocol";
import type { DepositView, WorldState } from "./worldState";

/**
 * What the map draws of the world's deposits, one per tile.
 *
 * The level is the simulation's reading of what is left; an exhausted deposit (level 0) is not
 * drawn. On the rare tile a guarantee laid a second deposit on, the richer one is drawn.
 */
export const mapDepositsOf = (world: WorldState): MapDeposit[] => {
  const byTile = new Map<number, DepositView>();
  for (const deposit of world.deposits.values()) {
    if (deposit.level === 0 || deposit.resource === null || !isDepositResource(deposit.resource)) {
      continue;
    }
    const drawn = byTile.get(deposit.tile);
    if (!drawn || deposit.level > drawn.level) {
      byTile.set(deposit.tile, deposit);
    }
  }
  return [...byTile.values()].map(({ tile, resource, level }) => ({
    tile,
    resource: resource as MapDeposit["resource"],
    level: Math.min(level, DEPOSIT_LEVELS) as DepositLevel,
  }));
};
