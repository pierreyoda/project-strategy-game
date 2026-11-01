import { RESOURCE_NAMES, formatChange, formatQuantity } from "../../resources";
import type { TileDeposit } from "../../state/tileDetails";

/**
 * A deposit in a few words, for the hover line of the resource lens: what it holds and how richly,
 * as the lens's pips show it - trace, deposit, rich - then its figures and how long they last.
 */
export const depositSummary = ({ resource, level, left, recharge, turnsLeft }: TileDeposit): string => {
  const name = RESOURCE_NAMES[resource];
  const what =
    level === 0
      ? `Exhausted ${name.toLowerCase()} deposit`
      : level === 1
        ? `Trace of ${name.toLowerCase()}`
        : level === 3
          ? `Rich ${name.toLowerCase()} deposit`
          : `${name} deposit`;
  const figures = recharge ? `${formatChange(recharge)} per turn` : `${formatQuantity(left)} left`;
  const lasts = turnsLeft !== null ? ` · runs out in ${turnsLeft} turns` : "";
  return `${what} · ${figures}${lasts}`;
};
