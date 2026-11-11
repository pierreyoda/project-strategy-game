import { describe, expect, it } from "vitest";

import { TerrainKind } from "../../map/terrain";
import { Vegetation } from "../../map/vegetation";
import { nationColor } from "../nations";
import {
  type BuildingLevelMeta,
  BuildingState,
  type ProjectionMeta,
  TileFlags,
  UNKNOWN_BUILDING_KIND,
  type WorldSnapshotData,
} from "../session/protocol";
import { constructionsOf, mapEntitiesOf } from "./mapEntities";
import { applyProjection } from "./worldState";

const level = (sprite: string): BuildingLevelMeta => ({
  sprite,
  cost: {},
  turns: 3,
  maintenance: {},
  production: {},
});

/** A mine whose third level reuses the second's art, as a script may. */
const meta: ProjectionMeta = {
  version: 1,
  turn: 1,
  viewer: 0,
  activeNation: 0,
  nations: [],
  buildingTemplates: [
    { id: "mine", type: "mine", deposit: "metals", levels: [level("mine_1"), level("mine_2"), level("mine_2")] },
  ],
  unitTemplates: [],
};

const none = () => ({ ids: new Uint32Array(), tiles: new Uint32Array(), owners: new Uint16Array() });

/** Buildings as the simulation sends them: `[tile, owner, kind, level, state, turnsLeft, turnsTotal]`. */
const buildings = (...rows: [number, number, number, number, number, number, number][]) => ({
  ids: new Uint32Array(rows.map((_, id) => id)),
  tiles: new Uint32Array(rows.map(([tile]) => tile)),
  owners: new Uint16Array(rows.map(([, owner]) => owner)),
  kinds: new Uint8Array(rows.map(([, , kind]) => kind)),
  levels: new Uint8Array(rows.map(([, , , level]) => level)),
  states: new Uint8Array(rows.map(([, , , , state]) => state)),
  turnsLeft: new Uint16Array(rows.map(([, , , , , turnsLeft]) => turnsLeft)),
  turnsTotal: new Uint16Array(rows.map(([, , , , , , turnsTotal]) => turnsTotal)),
});

const { UnderConstruction, Upgrading } = BuildingState;

/**
 * A 5 × 1 strip: the viewer's mine going up, its level 2 mine being raised to 3, a rival's mine going
 * up - sent without its turns - the viewer's level 3 mine, and a building of a template nobody
 * registered.
 */
const snapshot = (): WorldSnapshotData => ({
  meta,
  columns: 5,
  rows: 1,
  terrain: new Uint8Array(5).fill(TerrainKind.Flat),
  vegetation: new Uint8Array(5).fill(Vegetation.None),
  elevations: new Int16Array(5),
  visibility: new Uint8Array(5).fill(2),
  tileOwner: new Uint16Array(5).fill(0xffff),
  tileSphere: new Uint16Array(5).fill(0xffff),
  tileFlags: new Uint8Array(5),
  units: {
    ...none(),
    companies: new Uint8Array(),
    suppressed: new Uint8Array(),
    lost: new Uint8Array(),
    kinds: new Uint8Array(),
    movementLeft: new Uint8Array(),
  },
  settlements: { ...none(), stages: new Uint8Array() },
  buildings: buildings(
    [0, 0, 0, 0, UnderConstruction, 2, 3],
    [1, 0, 0, 2, Upgrading, 1, 4],
    [2, 1, 0, 0, UnderConstruction, 0, 0],
    [3, 0, 0, 3, 0, 0, 0],
    [4, 0, UNKNOWN_BUILDING_KIND, 1, 0, 0, 0],
  ),
  deposits: {
    ...none(),
    resources: new Uint8Array(),
    levels: new Uint8Array(),
    quantities: new Uint16Array(),
    recharges: new Uint16Array(),
    turnsLeft: new Uint16Array(),
  },
});

const world = () => applyProjection(null, { kind: "full", snapshot: snapshot() }).state;

describe("buildings on the map", () => {
  it("are drawn with the sprite their template names for their level", () => {
    const drawn = mapEntitiesOf(world()).map(({ tile, sprite, ghost }) => ({ tile, base: sprite.base, ghost }));
    expect(drawn).toStrictEqual([
      // still going up: drawn as the level 1 it will be, faded
      { tile: 0, base: "mine_1.png", ghost: true },
      // being raised: drawn as the level it works at, not faded
      { tile: 1, base: "mine_2.png", ghost: false },
      // a rival's scaffolding shows as much as the viewer's
      { tile: 2, base: "mine_1.png", ghost: true },
      // a level may reuse another's art: the template's call, not a naming rule
      { tile: 3, base: "mine_2.png", ghost: false },
      // and a building nobody can name is left off the map rather than drawn as another
    ]);
  });

  it("carry their sprite and their template's level count to the panels", () => {
    const views = [...world().buildings.values()].map(({ sprite, levels }) => ({ sprite, levels }));
    expect(views).toStrictEqual([
      { sprite: "mine_1", levels: 3 },
      { sprite: "mine_2", levels: 3 },
      { sprite: "mine_1", levels: 3 },
      { sprite: "mine_2", levels: 3 },
      { sprite: null, levels: 0 },
    ]);
  });

  it("show the progress of the viewer's own works only", () => {
    expect(constructionsOf(world())).toStrictEqual([
      { id: 0, tile: 0, turnsLeft: 2, turnsTotal: 3 },
      { id: 1, tile: 1, turnsLeft: 1, turnsTotal: 4 },
    ]);
  });
});

describe("districts on the map", () => {
  it("are drawn where no building stands yet, faded while laid out, in their owner's colours", () => {
    const { District, DistrictWorks, Building } = TileFlags;
    const state = applyProjection(null, {
      kind: "full",
      snapshot: {
        ...snapshot(),
        tileOwner: new Uint16Array([0, 1, 0, 0, 0xffff]),
        tileFlags: new Uint8Array([District, District | DistrictWorks, District | Building, 0, 0]),
        buildings: buildings(),
      },
    }).state;

    const drawn = mapEntitiesOf(state).map(({ tile, sprite, token, tint, ghost }) => ({
      tile,
      base: sprite.base,
      token,
      tint,
      ghost,
    }));
    expect(drawn).toStrictEqual([
      { tile: 0, base: "district_empty.png", token: null, tint: nationColor(0), ghost: false },
      { tile: 1, base: "district_empty.png", token: null, tint: nationColor(1), ghost: true },
      // and the building on the third draws its own ground works
    ]);
  });
});

describe("units on the map", () => {
  /** A rival's infantry, battered, next to the viewer's own at full strength. */
  const withUnits = (): WorldSnapshotData => ({
    ...snapshot(),
    meta: {
      ...meta,
      unitTemplates: [{ type: "infantry_division", movement: 2, sight: 2, companies: 4 }],
    },
    units: {
      ids: new Uint32Array([0, 1]),
      tiles: new Uint32Array([0, 1]),
      owners: new Uint16Array([0, 1]),
      companies: new Uint8Array([4, 4]),
      suppressed: new Uint8Array([0, 1]),
      lost: new Uint8Array([0, 2]),
      kinds: new Uint8Array([0, 0]),
      movementLeft: new Uint8Array([2, 2]),
    },
  });

  it("carry the strength the simulation sent, a rival's as much as the viewer's", () => {
    const state = applyProjection(null, { kind: "full", snapshot: withUnits() }).state;
    const strengths = [...state.units.values()].map(({ strength }) => strength);
    expect(strengths).toStrictEqual([
      { companies: 4, suppressed: 0, lost: 0 },
      { companies: 4, suppressed: 1, lost: 2 },
    ]);
    const counters = mapEntitiesOf(state)
      .filter(({ strength }) => strength !== undefined)
      .map(({ tile, strength }) => ({ tile, strength }));
    expect(counters).toStrictEqual([
      { tile: 0, strength: { companies: 4, suppressed: 0, lost: 0 } },
      { tile: 1, strength: { companies: 4, suppressed: 1, lost: 2 } },
    ]);
  });
});
