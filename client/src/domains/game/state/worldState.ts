import type { HexMapData, OffsetCoords } from "../../map/hexMap";
import { RESOURCES, type Resource } from "../resources";
import {
  type BuildingsBuffers,
  type DepositsBuffers,
  NO_TURNS_LEFT,
  type ProjectionMeta,
  type SettlementStage,
  UNKNOWN_BUILDING_KIND,
  UNKNOWN_UNIT_KIND,
  type UnitsBuffers,
  type WorldDeltaData,
  type WorldProjection,
  type WorldSnapshotData,
} from "../session/protocol";

export type UnitView = {
  id: number;
  /** Row-major tile index. */
  tile: number;
  owner: number;
  health: number;
  /** Its template's type (`infantry_division`...), `null` for one the simulation could not name. */
  type: string | null;
  /** Movement points per turn, and those left this turn. The simulation's figures, never ours. */
  movement: number;
  movementLeft: number;
  sight: number;
};

export type SettlementView = {
  id: number;
  tile: number;
  owner: number;
  /** How far it has grown, which is what the map draws it as. The simulation's call, never ours. */
  stage: SettlementStage;
};

export type BuildingView = {
  id: number;
  tile: number;
  owner: number;
  /** Its template's ID, what an order names it by; `null` for one the simulation could not name. */
  template: string | null;
  /** Its template's type (`mine`, `solar_plant`...), `null` for one the simulation could not name. */
  type: string | null;
  /** How far it has been upgraded: 0 while it is first being built. The simulation's call. */
  level: number;
  /**
   * What its template draws it with: the level it works at, or for one still being built the level it
   * will be. `null` for a building the simulation could not name.
   */
  sprite: string | null;
  /** How many levels its template has, 0 for an unknown one. */
  levels: number;
  /** `BuildingState` bits. */
  state: number;
  /** Turns its works have left, and take in all: zero without works, and for a rival's. */
  turnsLeft: number;
  turnsTotal: number;
};

export type DepositView = {
  id: number;
  tile: number;
  /** What it holds, `null` for a deposit the simulation could not name. */
  resource: Resource | null;
  /** 1 to `DEPOSIT_LEVELS` as the simulation reads it from what is left, 0 once exhausted. */
  level: number;
  /** The stock left, 0 for a deposit that springs back instead. */
  quantity: number;
  /** What it recharges per turn, 0 for a stock. */
  recharge: number;
  /** Turns its stock lasts at the rate it is drawn from, `null` while nothing draws on it. */
  turnsLeft: number | null;
};

/**
 * What the client holds of the world: a *projection*, never the truth.
 *
 * The simulation owns the state; this is the latest thing it agreed to show the viewing nation.
 * Nothing here is ever computed from a game rule - the moment it is, there are two simulations.
 */
export type WorldState = {
  meta: ProjectionMeta;
  columns: number;
  rows: number;
  /** `TerrainKind` values, for the whole map. */
  terrain: Uint8Array;
  /** `Vegetation` values, for the whole map, `UNKNOWN_VEGETATION` where never explored. */
  vegetation: Uint8Array;
  /** In meters, zeroed on tiles the viewer has never explored. */
  elevations: Int16Array;
  /** `TileVisibility` values. */
  visibility: Uint8Array;
  tileOwner: Uint16Array;
  /** Spheres of influence, as far as the viewer knows them. */
  tileSphere: Uint16Array;
  tileFlags: Uint8Array;
  units: Map<number, UnitView>;
  settlements: Map<number, SettlementView>;
  buildings: Map<number, BuildingView>;
  /** Every deposit the viewer has explored. None is ever removed: a deposit does not move. */
  deposits: Map<number, DepositView>;
};

export type AppliedProjection = {
  state: WorldState;
  /**
   * The tiles to redraw, or `null` when everything has to be.
   *
   * This is what a delta buys: moving one unit repaints two hexes instead of the map.
   */
  dirtyTiles: Uint32Array | null;
};

export const tileIndex = (state: Pick<WorldState, "columns">, { col, row }: OffsetCoords): number =>
  row * state.columns + col;

/** The view the existing map renderer takes, so it can draw a session's world unchanged. */
export const toHexMapData = (state: WorldState): HexMapData => ({
  columns: state.columns,
  rows: state.rows,
  elevations: state.elevations,
  terrain: state.terrain,
  vegetation: state.vegetation,
});

/** Fold a projection's units into a map of views, the kinds resolved against that projection's meta. */
const readUnits = (into: Map<number, UnitView>, buffers: UnitsBuffers, meta: ProjectionMeta) => {
  for (let index = 0; index < buffers.ids.length; index++) {
    const id = buffers.ids[index];
    const kind = buffers.kinds[index];
    const template = kind === UNKNOWN_UNIT_KIND ? undefined : meta.unitTemplates[kind];
    into.set(id, {
      id,
      tile: buffers.tiles[index],
      owner: buffers.owners[index],
      health: buffers.health[index],
      type: template?.type ?? null,
      movement: template?.movement ?? 0,
      movementLeft: buffers.movementLeft[index],
      sight: template?.sight ?? 0,
    });
  }
};

/** Fold a projection's deposits into a map of views. */
const readDeposits = (into: Map<number, DepositView>, buffers: DepositsBuffers) => {
  for (let index = 0; index < buffers.ids.length; index++) {
    const id = buffers.ids[index];
    const turnsLeft = buffers.turnsLeft[index];
    into.set(id, {
      id,
      tile: buffers.tiles[index],
      resource: RESOURCES[buffers.resources[index]] ?? null,
      level: buffers.levels[index],
      quantity: buffers.quantities[index],
      recharge: buffers.recharges[index],
      turnsLeft: turnsLeft === NO_TURNS_LEFT ? null : turnsLeft,
    });
  }
};

/** Fold a projection's buildings into a map of views, the kinds resolved against that projection's meta. */
const readBuildings = (into: Map<number, BuildingView>, buffers: BuildingsBuffers, meta: ProjectionMeta) => {
  for (let index = 0; index < buffers.ids.length; index++) {
    const id = buffers.ids[index];
    const kind = buffers.kinds[index];
    const template = kind === UNKNOWN_BUILDING_KIND ? undefined : meta.buildingTemplates[kind];
    const level = buffers.levels[index];
    into.set(id, {
      id,
      tile: buffers.tiles[index],
      owner: buffers.owners[index],
      template: template?.id ?? null,
      type: template?.type ?? null,
      level,
      // level 0 is a building still going up: drawn as the level 1 it will be
      sprite: template?.levels[Math.max(level, 1) - 1]?.sprite ?? null,
      levels: template?.levels.length ?? 0,
      state: buffers.states[index],
      turnsLeft: buffers.turnsLeft[index],
      turnsTotal: buffers.turnsTotal[index],
    });
  }
};

const fromSnapshot = (snapshot: WorldSnapshotData): WorldState => {
  const units = new Map<number, UnitView>();
  readUnits(units, snapshot.units, snapshot.meta);
  const settlements = new Map<number, SettlementView>();
  for (let index = 0; index < snapshot.settlements.ids.length; index++) {
    const id = snapshot.settlements.ids[index];
    settlements.set(id, {
      id,
      tile: snapshot.settlements.tiles[index],
      owner: snapshot.settlements.owners[index],
      stage: snapshot.settlements.stages[index] as SettlementStage,
    });
  }
  const buildings = new Map<number, BuildingView>();
  readBuildings(buildings, snapshot.buildings, snapshot.meta);
  const deposits = new Map<number, DepositView>();
  readDeposits(deposits, snapshot.deposits);
  return {
    meta: snapshot.meta,
    columns: snapshot.columns,
    rows: snapshot.rows,
    terrain: snapshot.terrain,
    vegetation: snapshot.vegetation,
    elevations: snapshot.elevations,
    visibility: snapshot.visibility,
    tileOwner: snapshot.tileOwner,
    tileSphere: snapshot.tileSphere,
    tileFlags: snapshot.tileFlags,
    units,
    settlements,
    buildings,
    deposits,
  };
};

const patch = (state: WorldState, delta: WorldDeltaData): WorldState => {
  // the tile arrays of a delta are parallel to its indices, not to the map
  for (let index = 0; index < delta.tileIndices.length; index++) {
    const tile = delta.tileIndices[index];
    state.terrain[tile] = delta.terrain[index];
    state.vegetation[tile] = delta.vegetation[index];
    state.elevations[tile] = delta.elevations[index];
    state.visibility[tile] = delta.visibility[index];
    state.tileOwner[tile] = delta.tileOwner[index];
    state.tileSphere[tile] = delta.tileSphere[index];
    state.tileFlags[tile] = delta.tileFlags[index];
  }

  for (const id of delta.unitsRemoved) {
    state.units.delete(id);
  }
  readUnits(state.units, delta.units, delta.meta);

  for (const id of delta.settlementsRemoved) {
    state.settlements.delete(id);
  }
  for (let index = 0; index < delta.settlements.ids.length; index++) {
    const id = delta.settlements.ids[index];
    state.settlements.set(id, {
      id,
      tile: delta.settlements.tiles[index],
      owner: delta.settlements.owners[index],
      stage: delta.settlements.stages[index] as SettlementStage,
    });
  }

  for (const id of delta.buildingsRemoved) {
    state.buildings.delete(id);
  }
  readBuildings(state.buildings, delta.buildings, delta.meta);
  readDeposits(state.deposits, delta.deposits);

  state.meta = delta.meta;
  return state;
};

/**
 * Fold a projection into what the client already holds.
 *
 * A full projection replaces everything; a delta patches the buffers in place and reports which
 * tiles moved. Which of the two arrives is the simulation's decision, so there is nothing to decide
 * here - only a delta that arrives with nothing to patch is a bug, and says so.
 */
export const applyProjection = (previous: WorldState | null, projection: WorldProjection): AppliedProjection => {
  if (projection.kind === "full") {
    return { state: fromSnapshot(projection.snapshot), dirtyTiles: null };
  }
  if (!previous) {
    throw new Error("a delta arrived before any snapshot: ask for a full projection first.");
  }
  return { state: patch(previous, projection.delta), dirtyTiles: projection.delta.tileIndices };
};
