import { type DepositLevel, type MapDeposit, isDepositResource } from "../../map/deposits";
import { DEPOSIT_LEVELS } from "../session/protocol";
import type { DepositView, WorldState } from "./worldState";

/**
 * What the map draws of the world's deposits, one per tile.
 *
 * The level is the simulation's reading of what is left. An exhausted deposit (level 0) is drawn
 * only for the turn it ran dry, when it is one of `justExhausted`, and not at all after that. On the
 * rare tile a guarantee laid a second deposit on, the richer one is drawn.
 */
export const mapDepositsOf = (world: WorldState, justExhausted: ReadonlySet<number> = new Set()): MapDeposit[] => {
  const built = new Set([...world.buildings.values()].map(({ tile }) => tile));
  const byTile = new Map<number, DepositView>();
  for (const deposit of world.deposits.values()) {
    if (deposit.resource === null || !isDepositResource(deposit.resource)) {
      continue;
    }
    if (deposit.level === 0 && !justExhausted.has(deposit.id)) {
      continue;
    }
    const drawn = byTile.get(deposit.tile);
    if (!drawn || deposit.level > drawn.level) {
      byTile.set(deposit.tile, deposit);
    }
  }
  return [...byTile.values()].map(({ tile, resource, level, turnsLeft }) => ({
    tile,
    resource: resource as MapDeposit["resource"],
    // the trace's art for one run dry, the last thing it looked like
    level: Math.min(Math.max(level, 1), DEPOSIT_LEVELS) as DepositLevel,
    depleting: level > 0 && turnsLeft !== null,
    exhausted: level === 0,
    covered: built.has(tile),
  }));
};
