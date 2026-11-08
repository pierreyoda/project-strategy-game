import { describe, expect, it } from "vitest";

import { companyStates, describeStrength } from "./entities";

describe("a unit's strength", () => {
  it("is drawn active companies first, then the suppressed, then the lost", () => {
    expect(companyStates({ companies: 5, suppressed: 2, lost: 1 })).toStrictEqual([
      "active",
      "active",
      "suppressed",
      "suppressed",
      "lost",
    ]);
    expect(companyStates({ companies: 3, suppressed: 0, lost: 0 })).toStrictEqual(["active", "active", "active"]);
    // broken: every company it has left is suppressed
    expect(companyStates({ companies: 2, suppressed: 1, lost: 1 })).toStrictEqual(["suppressed", "lost"]);
  });

  it("reads as the companies left, and those suppressed if any", () => {
    expect(describeStrength({ companies: 4, suppressed: 1, lost: 1 })).toBe("3 of 4 companies, 1 suppressed");
    expect(describeStrength({ companies: 3, suppressed: 0, lost: 0 })).toBe("3 of 3 companies");
    expect(describeStrength({ companies: 1, suppressed: 0, lost: 0 })).toBe("1 of 1 company");
  });
});
