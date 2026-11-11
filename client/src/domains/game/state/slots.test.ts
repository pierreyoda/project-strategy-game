import { describe, expect, it } from "vitest";

import { TerrainKind } from "../../map/terrain";
import { Vegetation } from "../../map/vegetation";
import { RESOURCES } from "../resources";
import {
  type BuildingLevelMeta,
  BuildingState,
  type ProjectionMeta,
  type ResourceAmounts,
  TileFlags,
  type WorldSnapshotData,
} from "../session/protocol";
import { slotFiguresOf, slotOf, tileDetailsOf } from "./tileDetails";
import { applyProjection } from "./worldState";

const VIEWER = 0;
const RIVAL = 1;
const NOBODY = 0xffff;

const level = (cost: ResourceAmounts, turns: number, production: ResourceAmounts): BuildingLevelMeta => ({
  sprite: "",
  cost,
  turns,
  maintenance: { credits: turns },
  production,
});

/** A mine that extracts metals, and a plant that extracts nothing. */
const meta: ProjectionMeta = {
  version: 1,
  turn: 5,
  viewer: VIEWER,
  activeNation: VIEWER,
  nations: [],
  buildingTemplates: [
    {
      id: "mine",
      type: "mine",
      deposit: "metals",
      levels: [level({ credits: 200 }, 3, { metals: 5 }), level({ credits: 300 }, 4, { metals: 8 })],
    },
    { id: "plant", type: "plant", deposit: null, levels: [level({ metals: 50 }, 2, { electricity: 3 })] },
  ],
  unitTemplates: [],
};

type Tile = {
  terrain?: TerrainKind;
  owner?: number;
  /** `[resource, level]` of the deposit on it. */
  deposit?: [(typeof RESOURCES)[number], number];
  /** `[kind, level, state, turnsLeft, turnsTotal]` of the building on it. */
  building?: [number, number, number, number, number];
  district?: "laid out" | "under way";
};

const flagsOf = ({ deposit, district }: Tile) =>
  (deposit ? TileFlags.Deposit : 0) |
  (district ? TileFlags.District : 0) |
  (district === "under way" ? TileFlags.DistrictWorks : 0);

/** A one-row world of the given tiles, every one of them in sight. */
const world = (...tiles: Tile[]) => {
  const columns = tiles.length;
  const built = tiles.flatMap((tile, index) => (tile.building ? [{ index, row: tile.building }] : []));
  const mined = tiles.flatMap((tile, index) => (tile.deposit ? [{ index, row: tile.deposit }] : []));
  const snapshot: WorldSnapshotData = {
    meta,
    columns,
    rows: 1,
    terrain: new Uint8Array(tiles.map(({ terrain = TerrainKind.Flat }) => terrain)),
    vegetation: new Uint8Array(columns).fill(Vegetation.None),
    elevations: new Int16Array(columns),
    visibility: new Uint8Array(columns).fill(2),
    tileOwner: new Uint16Array(tiles.map(({ owner = NOBODY }) => owner)),
    tileSphere: new Uint16Array(columns).fill(NOBODY),
    tileFlags: new Uint8Array(tiles.map(flagsOf)),
    units: {
      ids: new Uint32Array(),
      tiles: new Uint32Array(),
      owners: new Uint16Array(),
      companies: new Uint8Array(),
      suppressed: new Uint8Array(),
      lost: new Uint8Array(),
      kinds: new Uint8Array(),
      movementLeft: new Uint8Array(),
    },
    settlements: {
      ids: new Uint32Array(),
      tiles: new Uint32Array(),
      owners: new Uint16Array(),
      stages: new Uint8Array(),
    },
    buildings: {
      ids: new Uint32Array(built.map((_, id) => id)),
      tiles: new Uint32Array(built.map(({ index }) => index)),
      owners: new Uint16Array(built.map(({ index }) => tiles[index].owner ?? NOBODY)),
      kinds: new Uint8Array(built.map(({ row: [kind] }) => kind)),
      levels: new Uint8Array(built.map(({ row: [, buildingLevel] }) => buildingLevel)),
      states: new Uint8Array(built.map(({ row: [, , state] }) => state)),
      turnsLeft: new Uint16Array(built.map(({ row: [, , , turnsLeft] }) => turnsLeft)),
      turnsTotal: new Uint16Array(built.map(({ row: [, , , , turnsTotal] }) => turnsTotal)),
    },
    deposits: {
      ids: new Uint32Array(mined.map((_, id) => id)),
      tiles: new Uint32Array(mined.map(({ index }) => index)),
      resources: new Uint8Array(mined.map(({ row: [resource] }) => RESOURCES.indexOf(resource))),
      levels: new Uint8Array(mined.map(({ row: [, depositLevel] }) => depositLevel)),
      quantities: new Uint16Array(mined.map(({ row: [, depositLevel] }) => depositLevel * 100)),
      recharges: new Uint16Array(mined.length),
      turnsLeft: new Uint16Array(mined.length).fill(0xffff),
    },
  };
  return applyProjection(null, { kind: "full", snapshot }).state;
};

/** The slot of a world's only tile, and its figures. */
const slotOfOnly = (tile: Tile) => {
  const state = world(tile);
  const details = tileDetailsOf(state, 0);
  const slot = slotOf(details, VIEWER);
  return { slot, figures: slotFiguresOf(state, details, slot) };
};

const MINE = 0;
const PLANT = 1;
const { UnderConstruction, Upgrading, Halted } = BuildingState;

describe("a tile's building slot", () => {
  it("is free on the viewer's own land only, as the simulation rules", () => {
    expect(slotOfOnly({ owner: VIEWER, district: "laid out" }).slot).toStrictEqual({ state: "free" });
    expect(slotOfOnly({ district: "laid out" }).slot).toStrictEqual({
      state: "none",
      reason: "Only the nation's own land can be built on.",
    });
    expect(slotOfOnly({ owner: VIEWER, terrain: TerrainKind.ShallowWater }).slot).toStrictEqual({
      state: "none",
      reason: "Nothing can be built on water.",
    });
    expect(slotOfOnly({ owner: RIVAL }).slot.state).toBe("foreign");
  });

  it("takes a district laid out, or a deposit for an extractor", () => {
    expect(slotOfOnly({ owner: VIEWER }).slot).toStrictEqual({
      state: "none",
      reason: "No district is laid out here: without one, only an extractor on its deposit can be built.",
    });
    expect(slotOfOnly({ owner: VIEWER, deposit: ["metals", 2] }).slot).toStrictEqual({ state: "free" });
    expect(slotOfOnly({ owner: VIEWER, deposit: ["metals", 0] }).slot.state, "run dry").toBe("none");
    expect(slotOfOnly({ owner: VIEWER, district: "under way", deposit: ["metals", 2] }).slot).toStrictEqual({
      state: "none",
      reason: "A district is being laid out here: it takes a building once it is done.",
    });
  });

  it("shows works under way from the building's state, built otherwise", () => {
    const going = slotOfOnly({ owner: VIEWER, building: [PLANT, 0, UnderConstruction, 1, 2] }).slot;
    expect(going).toMatchObject({ state: "construction", turnsLeft: 1, turnsTotal: 2 });

    for (const state of [0, Upgrading, Halted]) {
      const standing = slotOfOnly({ owner: VIEWER, building: [MINE, 1, state, 0, 0] }).slot;
      expect(standing.state).toBe("built");
    }
  });
});

describe("a free slot's options", () => {
  const options = (tile: Tile) => slotOfOnly({ owner: VIEWER, ...tile }).figures.options;
  const templates = (tile: Tile) => options(tile)?.map(({ template }) => template);

  it("offer an extractor on a deposit of what it extracts, favoured, with its first level's cost", () => {
    expect(options({ deposit: ["metals", 2], district: "laid out" })).toStrictEqual([
      { template: "mine", type: "mine", sprite: "", cost: { credits: 200 }, turns: 3, favoured: "Deposit" },
      { template: "plant", type: "plant", sprite: "", cost: { metals: 50 }, turns: 2, favoured: undefined },
    ]);
  });

  it("leave an extractor out where there is nothing for it to extract", () => {
    expect(templates({ district: "laid out" })).toStrictEqual(["plant"]);
    expect(templates({ deposit: ["oil", 3], district: "laid out" })).toStrictEqual(["plant"]);
    expect(templates({ deposit: ["metals", 0], district: "laid out" }), "run dry").toStrictEqual(["plant"]);
  });

  it("leave out what extracts nothing where no district is laid out", () => {
    expect(templates({ deposit: ["metals", 2] })).toStrictEqual(["mine"]);
    expect(templates({ deposit: ["oil", 3] }), "nothing extracts oil").toStrictEqual([]);
  });
});

describe("a building's figures", () => {
  it("are its level's, and the next level's to upgrade to", () => {
    const { figures } = slotOfOnly({ owner: VIEWER, building: [MINE, 1, 0, 0, 0] });
    expect(figures).toStrictEqual({
      output: { metals: 5 },
      upkeep: { credits: 3 },
      upgrade: { cost: { credits: 300 }, turns: 4 },
      spent: undefined,
      endsTurn: undefined,
    });
    expect(slotOfOnly({ owner: VIEWER, building: [MINE, 2, 0, 0, 0] }).figures.upgrade, "at its last").toBeUndefined();
  });

  it("tell what works under way were paid and when they complete", () => {
    // turn 5, two turns left: this turn and the next, which it completes at the end of
    const building = slotOfOnly({ owner: VIEWER, building: [PLANT, 0, UnderConstruction, 2, 2] }).figures;
    expect(building).toMatchObject({ spent: { metals: 50 }, endsTurn: 6 });

    const upgrade = slotOfOnly({ owner: VIEWER, building: [MINE, 1, Upgrading, 1, 4] }).figures;
    expect(upgrade).toMatchObject({ output: { metals: 5 }, spent: { credits: 300 }, endsTurn: 5 });
  });
});
