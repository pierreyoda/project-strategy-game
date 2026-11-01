import { describe, expect, it } from "vitest";

import { TerrainKind, UNKNOWN_TERRAIN } from "../../map/terrain";
import { UNKNOWN_VEGETATION, Vegetation, groundName } from "../../map/vegetation";
import type { ProjectionMeta, WorldDeltaData, WorldSnapshotData } from "../session/protocol";
import { tileDetailsOf } from "./tileDetails";
import { applyProjection, toHexMapData } from "./worldState";

const meta: ProjectionMeta = {
  version: 1,
  turn: 1,
  viewer: 0,
  activeNation: 0,
  nations: [],
  buildingTypes: [],
  unitTemplates: [],
};

const noUnits = () => ({
  ids: new Uint32Array(),
  tiles: new Uint32Array(),
  owners: new Uint16Array(),
  health: new Uint16Array(),
  kinds: new Uint8Array(),
  movementLeft: new Uint8Array(),
});
const noSettlements = () => ({
  ids: new Uint32Array(),
  tiles: new Uint32Array(),
  owners: new Uint16Array(),
  stages: new Uint8Array(),
});
const noDeposits = () => ({
  ids: new Uint32Array(),
  tiles: new Uint32Array(),
  resources: new Uint8Array(),
  levels: new Uint8Array(),
  quantities: new Uint16Array(),
  recharges: new Uint16Array(),
  turnsLeft: new Uint16Array(),
});
const noBuildings = () => ({
  ids: new Uint32Array(),
  tiles: new Uint32Array(),
  owners: new Uint16Array(),
  kinds: new Uint8Array(),
  levels: new Uint8Array(),
});

/** A 3 × 1 map: a forest, bare hills, and an unexplored tile. */
const snapshot = (): WorldSnapshotData => ({
  meta,
  columns: 3,
  rows: 1,
  terrain: new Uint8Array([TerrainKind.Flat, TerrainKind.Hills, UNKNOWN_TERRAIN]),
  vegetation: new Uint8Array([Vegetation.Forest, Vegetation.None, UNKNOWN_VEGETATION]),
  elevations: new Int16Array([200, 900, 0]),
  visibility: new Uint8Array([2, 2, 0]),
  tileOwner: new Uint16Array([0xffff, 0xffff, 0xffff]),
  tileSphere: new Uint16Array([0xffff, 0xffff, 0xffff]),
  tileFlags: new Uint8Array(3),
  units: noUnits(),
  settlements: noSettlements(),
  buildings: noBuildings(),
  deposits: noDeposits(),
});

describe("vegetation in the world state", () => {
  it("comes in with a snapshot and reaches the map renderer", () => {
    const { state } = applyProjection(null, { kind: "full", snapshot: snapshot() });
    expect([...state.vegetation]).toStrictEqual([Vegetation.Forest, Vegetation.None, UNKNOWN_VEGETATION]);
    expect(toHexMapData(state).vegetation).toBe(state.vegetation);
  });

  it("is patched in by a delta exploring a tile", () => {
    const { state } = applyProjection(null, { kind: "full", snapshot: snapshot() });
    const delta: WorldDeltaData = {
      meta: { ...meta, version: 2 },
      tileIndices: new Uint32Array([2]),
      terrain: new Uint8Array([TerrainKind.Hills]),
      vegetation: new Uint8Array([Vegetation.Forest]),
      elevations: new Int16Array([800]),
      visibility: new Uint8Array([2]),
      tileOwner: new Uint16Array([0xffff]),
      tileSphere: new Uint16Array([0xffff]),
      tileFlags: new Uint8Array(1),
      units: noUnits(),
      unitsRemoved: new Uint32Array(),
      settlements: noSettlements(),
      settlementsRemoved: new Uint32Array(),
      buildings: noBuildings(),
      buildingsRemoved: new Uint32Array(),
      deposits: noDeposits(),
    };
    const patched = applyProjection(state, { kind: "delta", delta });
    expect(patched.state.vegetation[2]).toBe(Vegetation.Forest);
    expect(tileDetailsOf(patched.state, 2).terrainName).toBe("Forested hills");
  });
});

describe("tile details", () => {
  it("name the forest over the terrain", () => {
    const { state } = applyProjection(null, { kind: "full", snapshot: snapshot() });
    expect(tileDetailsOf(state, 0)).toMatchObject({ terrainName: "Forest", forest: true });
    expect(tileDetailsOf(state, 1)).toMatchObject({ terrainName: "Hills", forest: false });
    expect(tileDetailsOf(state, 2)).toMatchObject({ terrainName: "Unexplored", forest: false });
  });

  it("fall back to the terrain's name with the forest after it", () => {
    expect(groundName(TerrainKind.Mountains, true)).toBe("Mountains (forest)");
    expect(groundName(TerrainKind.Flat, false)).toBe("Flat");
  });
});
