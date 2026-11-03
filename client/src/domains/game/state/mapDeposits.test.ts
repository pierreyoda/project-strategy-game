import { describe, expect, it } from "vitest";

import type { DepositView, WorldState } from "./worldState";
import { mapDepositsOf } from "./mapDeposits";

type Laid = Omit<DepositView, "id" | "quantity" | "recharge" | "turnsLeft"> & Partial<Pick<DepositView, "turnsLeft">>;

/** A world holding these deposits, ids in order from 0, and buildings on `builtOn`'s tiles. */
const world = (deposits: Laid[], builtOn: number[] = []) =>
  ({
    deposits: new Map(
      deposits.map((deposit, id) => [id, { id, quantity: 100, recharge: 0, turnsLeft: null, ...deposit }]),
    ),
    buildings: new Map(builtOn.map((tile, id) => [id, { id, tile }])),
  }) as unknown as WorldState;

describe("the deposits the map draws", () => {
  it("are one per tile, the richer one where a tile holds two", () => {
    const drawn = mapDepositsOf(
      world([
        { tile: 3, resource: "metals", level: 1 },
        { tile: 3, resource: "water", level: 3 },
        { tile: 8, resource: "oil", level: 2 },
      ]),
    );
    expect(drawn).toStrictEqual([
      { tile: 3, resource: "water", level: 3, depleting: false, exhausted: false, covered: false },
      { tile: 8, resource: "oil", level: 2, depleting: false, exhausted: false, covered: false },
    ]);
  });

  it("are depleting while something draws on them", () => {
    const drawn = mapDepositsOf(
      world([
        { tile: 5, resource: "oil", level: 2, turnsLeft: 12 },
        { tile: 6, resource: "metals", level: 1 },
      ]),
    );
    expect(drawn.map(({ tile, depleting }) => ({ tile, depleting }))).toStrictEqual([
      { tile: 5, depleting: true },
      { tile: 6, depleting: false },
    ]);
  });

  it("leave out the exhausted and what has no art", () => {
    const drawn = mapDepositsOf(
      world([
        { tile: 1, resource: "uranium", level: 0 },
        { tile: 2, resource: "food", level: 2 },
        { tile: 4, resource: null, level: 2 },
      ]),
    );
    expect(drawn).toStrictEqual([]);
  });

  it("keep one that ran dry this turn, at its trace's art, until the next", () => {
    const laid = world([
      { tile: 1, resource: "metals", level: 0, turnsLeft: 0 },
      { tile: 2, resource: "oil", level: 0 },
    ]);
    expect(mapDepositsOf(laid, new Set([0]))).toStrictEqual([
      { tile: 1, resource: "metals", level: 1, depleting: false, exhausted: true, covered: false },
    ]);
    expect(mapDepositsOf(laid, new Set())).toStrictEqual([]);
  });

  it("say which ones a building covers", () => {
    const drawn = mapDepositsOf(
      world(
        [
          { tile: 4, resource: "metals", level: 2, turnsLeft: 30 },
          { tile: 7, resource: "water", level: 3 },
        ],
        [4],
      ),
    );
    expect(drawn.map(({ tile, covered }) => ({ tile, covered }))).toStrictEqual([
      { tile: 4, covered: true },
      { tile: 7, covered: false },
    ]);
  });
});
