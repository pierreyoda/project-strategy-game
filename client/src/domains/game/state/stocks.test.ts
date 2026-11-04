import { describe, expect, it } from "vitest";

import { netIncome } from "../resources";
import type { ProjectionMeta } from "../session/protocol";
import { stocksOf } from "./stocks";

/** A projection's meta: the viewer, nation 0, holding `resources`, and a rival whose stockpiles are not shown. */
const meta = (resources: ProjectionMeta["nations"][number]["resources"]) =>
  ({
    nations: [
      { id: 0, name: "Viewer", resources },
      { id: 1, name: "Rival", resources: [] },
    ],
  }) as unknown as ProjectionMeta;

describe("the viewer's stocks", () => {
  it("take the turn's forecast as it comes, buildings and all", () => {
    // +2 water of income and +3 from a pump; 12 credits of income, 1 of a pump's upkeep
    const stocks = stocksOf(
      meta([
        { resource: "water", quantity: 40, income: 5, depletion: 0 },
        { resource: "credits", quantity: 100, income: 12, depletion: 1 },
      ]),
      0,
    );
    expect(stocks.water).toStrictEqual({ quantity: 40, income: 5, depletion: 0 });
    expect(netIncome(stocks.credits)).toBe(11);
  });

  it("hold every resource, at nothing where the projection says nothing", () => {
    const stocks = stocksOf(meta([{ resource: "metals", quantity: 8, income: 4, depletion: 0 }]), 0);
    expect(Object.keys(stocks)).toHaveLength(7);
    expect(stocks.uranium).toStrictEqual({ quantity: 0, income: 0, depletion: 0 });
  });

  it("are empty before a game, and for a viewer that is not a nation", () => {
    expect(netIncome(stocksOf(null, 0).credits)).toBe(0);
    expect(stocksOf(meta([{ resource: "food", quantity: 3, income: 1, depletion: 0 }]), 7).food.quantity).toBe(0);
  });
});
