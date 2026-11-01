import { describe, expect, it } from "vitest";

import type { DepositView, WorldState } from "./worldState";
import { mapDepositsOf } from "./mapDeposits";

const world = (
  ...deposits: (Omit<DepositView, "id" | "quantity" | "recharge" | "turnsLeft"> &
    Partial<Pick<DepositView, "turnsLeft">>)[]
) =>
  ({
    deposits: new Map(
      deposits.map((deposit, id) => [id, { id, quantity: 100, recharge: 0, turnsLeft: null, ...deposit }]),
    ),
  }) as unknown as WorldState;

describe("the deposits the map draws", () => {
  it("are one per tile, the richer one where a tile holds two", () => {
    const drawn = mapDepositsOf(
      world(
        { tile: 3, resource: "metals", level: 1 },
        { tile: 3, resource: "water", level: 3 },
        { tile: 8, resource: "oil", level: 2 },
      ),
    );
    expect(drawn).toStrictEqual([
      { tile: 3, resource: "water", level: 3, depleting: false },
      { tile: 8, resource: "oil", level: 2, depleting: false },
    ]);
  });

  it("are depleting while something draws on them", () => {
    const drawn = mapDepositsOf(
      world({ tile: 5, resource: "oil", level: 2, turnsLeft: 12 }, { tile: 6, resource: "metals", level: 1 }),
    );
    expect(drawn.map(({ tile, depleting }) => ({ tile, depleting }))).toStrictEqual([
      { tile: 5, depleting: true },
      { tile: 6, depleting: false },
    ]);
  });

  it("leave out the exhausted and what has no art", () => {
    const drawn = mapDepositsOf(
      world(
        { tile: 1, resource: "uranium", level: 0 },
        { tile: 2, resource: "food", level: 2 },
        { tile: 4, resource: null, level: 2 },
      ),
    );
    expect(drawn).toStrictEqual([]);
  });
});
