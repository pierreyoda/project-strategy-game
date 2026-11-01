import { describe, expect, it } from "vitest";

import { TerrainKind } from "../../map/terrain";
import { Vegetation } from "../../map/vegetation";
import { RESOURCES } from "../resources";
import {
  type DepositsBuffers,
  NO_TURNS_LEFT,
  type ProjectionMeta,
  TileFlags,
  type WorldDeltaData,
  type WorldSnapshotData,
} from "../session/protocol";
import { tileDetailsOf } from "./tileDetails";
import { applyProjection } from "./worldState";

const meta: ProjectionMeta = {
  version: 1,
  turn: 1,
  viewer: 0,
  activeNation: 0,
  nations: [],
  buildingTypes: [],
  unitTemplates: [],
};

const none = () => ({ ids: new Uint32Array(), tiles: new Uint32Array(), owners: new Uint16Array() });
const noUnits = () => ({
  ...none(),
  health: new Uint16Array(),
  kinds: new Uint8Array(),
  movementLeft: new Uint8Array(),
});
const noSettlements = () => ({ ...none(), stages: new Uint8Array() });
const noBuildings = () => ({ ...none(), kinds: new Uint8Array(), levels: new Uint8Array() });

/** Deposits as the simulation sends them: `[tile, resource, level, quantity, recharge, turnsLeft]`. */
const deposits = (...rows: [number, number, number, number, number, number][]): DepositsBuffers => ({
  ids: new Uint32Array(rows.map((_, id) => id)),
  tiles: new Uint32Array(rows.map(([tile]) => tile)),
  resources: new Uint8Array(rows.map(([, resource]) => resource)),
  levels: new Uint8Array(rows.map(([, , level]) => level)),
  quantities: new Uint16Array(rows.map(([, , , quantity]) => quantity)),
  recharges: new Uint16Array(rows.map(([, , , , recharge]) => recharge)),
  turnsLeft: new Uint16Array(rows.map(([, , , , , turnsLeft]) => turnsLeft)),
});

const resource = (name: (typeof RESOURCES)[number]) => RESOURCES.indexOf(name);

/** A 3 × 1 map: metals being worked on the first tile, water on the second, the third unexplored. */
const snapshot = (): WorldSnapshotData => ({
  meta,
  columns: 3,
  rows: 1,
  terrain: new Uint8Array([TerrainKind.Flat, TerrainKind.Hills, TerrainKind.Flat]),
  vegetation: new Uint8Array([Vegetation.None, Vegetation.None, Vegetation.None]),
  elevations: new Int16Array([200, 900, 0]),
  visibility: new Uint8Array([2, 2, 0]),
  tileOwner: new Uint16Array([0xffff, 0xffff, 0xffff]),
  tileSphere: new Uint16Array([0xffff, 0xffff, 0xffff]),
  tileFlags: new Uint8Array([TileFlags.Deposit, TileFlags.Deposit, 0]),
  units: noUnits(),
  settlements: noSettlements(),
  buildings: noBuildings(),
  deposits: deposits([0, resource("metals"), 2, 340, 0, 113], [1, resource("water"), 3, 0, 9, NO_TURNS_LEFT]),
});

describe("deposits in the world state", () => {
  it("come in with a snapshot, their resource named and their figures kept", () => {
    const { state } = applyProjection(null, { kind: "full", snapshot: snapshot() });
    expect([...state.deposits.values()]).toStrictEqual([
      { id: 0, tile: 0, resource: "metals", level: 2, quantity: 340, recharge: 0, turnsLeft: 113 },
      { id: 1, tile: 1, resource: "water", level: 3, quantity: 0, recharge: 9, turnsLeft: null },
    ]);
  });

  it("reach the tile panels", () => {
    const { state } = applyProjection(null, { kind: "full", snapshot: snapshot() });
    expect(tileDetailsOf(state, 0).deposit).toStrictEqual({
      resource: "metals",
      left: 340,
      recharge: 0,
      level: 2,
      turnsLeft: 113,
    });
    expect(tileDetailsOf(state, 1).deposit?.recharge).toBe(9);
    expect(tileDetailsOf(state, 2).deposit).toBeUndefined();
  });

  it("are added by a delta exploring their tile, and kept", () => {
    const { state } = applyProjection(null, { kind: "full", snapshot: snapshot() });
    const delta: WorldDeltaData = {
      meta: { ...meta, version: 2 },
      tileIndices: new Uint32Array([2]),
      terrain: new Uint8Array([TerrainKind.Flat]),
      vegetation: new Uint8Array([Vegetation.None]),
      elevations: new Int16Array([150]),
      visibility: new Uint8Array([2]),
      tileOwner: new Uint16Array([0xffff]),
      tileSphere: new Uint16Array([0xffff]),
      tileFlags: new Uint8Array([TileFlags.Deposit]),
      units: noUnits(),
      unitsRemoved: new Uint32Array(),
      settlements: noSettlements(),
      settlementsRemoved: new Uint32Array(),
      buildings: noBuildings(),
      buildingsRemoved: new Uint32Array(),
      deposits: { ...deposits([2, resource("uranium"), 1, 90, 0, NO_TURNS_LEFT]), ids: new Uint32Array([7]) },
    };
    const patched = applyProjection(state, { kind: "delta", delta }).state;
    expect(patched.deposits.size).toBe(3);
    expect(tileDetailsOf(patched, 2).deposit).toMatchObject({ resource: "uranium", level: 1, left: 90 });
  });
});
