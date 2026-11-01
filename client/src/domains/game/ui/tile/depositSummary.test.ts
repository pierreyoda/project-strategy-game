import { describe, expect, it } from "vitest";

import { depositSummary } from "./depositSummary";

describe("a deposit's summary", () => {
  it("names its richness the way the lens's pips show it", () => {
    expect(depositSummary({ resource: "oil", level: 1, left: 80, turnsLeft: null })).toBe("Trace of oil · 80 left");
    expect(depositSummary({ resource: "oil", level: 2, left: 340, turnsLeft: null })).toBe("Oil deposit · 340 left");
    expect(depositSummary({ resource: "metals", level: 3, left: 1200, turnsLeft: null })).toBe(
      "Rich metals deposit · 1,200 left",
    );
  });

  it("gives a spring's recharge in place of a stock", () => {
    expect(depositSummary({ resource: "water", level: 3, left: 0, recharge: 11, turnsLeft: null })).toBe(
      "Rich water deposit · +11 per turn",
    );
  });

  it("says when a deposit drawn on runs out", () => {
    expect(depositSummary({ resource: "uranium", level: 1, left: 24, turnsLeft: 6 })).toBe(
      "Trace of uranium · 24 left · runs out in 6 turns",
    );
    expect(depositSummary({ resource: "oil", level: 0, left: 0, turnsLeft: null })).toBe(
      "Exhausted oil deposit · 0 left",
    );
  });
});
