import { describe, expect, it } from "vitest";

import { createHexLayout, hexCenter, hexCorners, neighborAcross } from "./hexLayout";

const layout = createHexLayout(32);
const map = { columns: 10, rows: 6 };

describe("the neighbor across an edge", () => {
  it.each([
    ["an even row", { col: 4, row: 2 }],
    ["an odd row", { col: 4, row: 3 }],
  ])("lies beyond the middle of that edge, on %s", (_, hex) => {
    const center = hexCenter(layout, hex);
    const corners = hexCorners(layout);
    for (let edge = 0; edge < 6; edge++) {
      const next = (edge + 1) % 6;
      // the middle of the edge, halfway to the neighbor's center
      const middle = {
        x: (corners[2 * edge] + corners[2 * next]) / 2,
        y: (corners[2 * edge + 1] + corners[2 * next + 1]) / 2,
      };
      const neighbor = neighborAcross(map, hex, edge);
      expect(neighbor).not.toBeNull();
      const across = hexCenter(layout, neighbor!);
      expect(across.x - center.x).toBeCloseTo(2 * middle.x);
      expect(across.y - center.y).toBeCloseTo(2 * middle.y);
    }
  });

  it("wraps around east-west and stops at the poles", () => {
    expect(neighborAcross(map, { col: 9, row: 2 }, 1)).toStrictEqual({ col: 0, row: 2 });
    expect(neighborAcross(map, { col: 0, row: 2 }, 4)).toStrictEqual({ col: 9, row: 2 });
    expect(neighborAcross(map, { col: 3, row: 0 }, 0)).toBeNull();
    expect(neighborAcross(map, { col: 3, row: 5 }, 2)).toBeNull();
  });
});
