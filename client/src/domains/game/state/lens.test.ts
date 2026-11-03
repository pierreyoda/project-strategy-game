import { describe, expect, it } from "vitest";

import type { MapDeposit } from "../../map/deposits";
import { NO_NATION, TileVisibility } from "../session/protocol";
import { NO_LENS, filteredDeposits, knownNations, lensFiltered, lensForKey, lensToggled } from "./lens";
import type { WorldState } from "./worldState";

describe("switching lenses", () => {
  it("turns a lens on, over any other, and off again", () => {
    const resources = lensToggled(NO_LENS, "resources");
    expect(resources).toStrictEqual({ kind: "resources", filter: "all" });
    expect(lensToggled(resources, "political")).toStrictEqual({ kind: "political" });
    expect(lensToggled(resources, "resources")).toStrictEqual(NO_LENS);
  });

  it("filters the resource lens only", () => {
    const oil = lensFiltered({ kind: "resources", filter: "all" }, "oil");
    expect(oil).toStrictEqual({ kind: "resources", filter: "oil" });
    expect(lensFiltered(oil, "all")).toStrictEqual({ kind: "resources", filter: "all" });
    expect(lensFiltered({ kind: "political" }, "oil")).toStrictEqual({ kind: "political" });
    expect(lensFiltered(NO_LENS, "water")).toStrictEqual(NO_LENS);
  });

  it("opens the resource lens on every resource, whatever it was narrowed to before", () => {
    const narrowed = lensFiltered(lensToggled(NO_LENS, "resources"), "uranium");
    expect(lensToggled(lensToggled(narrowed, "resources"), "resources")).toStrictEqual({
      kind: "resources",
      filter: "all",
    });
  });
});

describe("the lens hotkeys", () => {
  const press = (key: string, modifiers: Partial<Record<"ctrlKey" | "metaKey" | "altKey", boolean>> = {}) =>
    lensForKey({ key, ctrlKey: false, metaKey: false, altKey: false, ...modifiers });

  it("are the letters the tooltips show, whatever their case", () => {
    expect(press("r")).toBe("resources");
    expect(press("R")).toBe("resources");
    expect(press("p")).toBe("political");
    expect(press("x")).toBeNull();
    expect(press("Escape")).toBeNull();
  });

  it("leave the application's shortcuts alone", () => {
    expect(press("r", { metaKey: true })).toBeNull();
    expect(press("r", { ctrlKey: true })).toBeNull();
    expect(press("p", { altKey: true })).toBeNull();
  });
});

describe("the deposits the resource lens shows", () => {
  const deposit = (tile: number, resource: MapDeposit["resource"], extra: Partial<MapDeposit> = {}): MapDeposit => ({
    tile,
    resource,
    level: 2,
    depleting: false,
    exhausted: false,
    covered: false,
    ...extra,
  });
  const deposits = [
    deposit(1, "water", { level: 3 }),
    deposit(4, "oil", { level: 1, depleting: true }),
    // a building's: the lens shows its chip all the same
    deposit(9, "oil", { covered: true }),
    deposit(12, "metals", { level: 1, exhausted: true }),
  ];

  it("are those of the resource it is narrowed to, or all of them", () => {
    expect(filteredDeposits(deposits, "oil").map(({ tile }) => tile)).toStrictEqual([4, 9]);
    expect(filteredDeposits(deposits, "uranium")).toStrictEqual([]);
  });

  it("leave out one that ran dry, which has nothing left to show", () => {
    expect(filteredDeposits(deposits, "all").map(({ tile }) => tile)).toStrictEqual([1, 4, 9]);
    expect(filteredDeposits(deposits, "metals")).toStrictEqual([]);
  });
});

describe("the nations the political lens names", () => {
  const { Unexplored, Fogged, Visible } = TileVisibility;
  const world = (tiles: [visibility: number, owner: number, sphere: number][]) =>
    ({
      visibility: Uint8Array.from(tiles.map(([visibility]) => visibility)),
      tileOwner: Uint16Array.from(tiles.map(([, owner]) => owner)),
      tileSphere: Uint16Array.from(tiles.map(([, , sphere]) => sphere)),
      meta: {
        nations: [0, 1, 2, 3, 4].map((id) => ({ id, name: `Nation ${id + 1}`, resources: [] })),
      },
    }) as unknown as WorldState;

  it("are the viewer's own first, then those seen on the map, by id", () => {
    const known = knownNations(
      world([
        [Visible, 3, 3],
        [Fogged, 1, NO_NATION],
        // never explored: who holds it is not the viewer's to know
        [Unexplored, 4, NO_NATION],
        // a sphere reaching into view gives its nation away
        [Unexplored, NO_NATION, 0],
      ]),
      2,
    );
    expect(known.map(({ id }) => id)).toStrictEqual([2, 0, 1, 3]);
  });

  it("are the viewer alone before anyone else is met", () => {
    expect(knownNations(world([[Visible, NO_NATION, NO_NATION]]), 1).map(({ id }) => id)).toStrictEqual([1]);
  });
});
