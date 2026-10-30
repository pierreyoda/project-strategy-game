import type { HexMapData, OffsetCoords } from "../../map/hexMap";
import type { ProjectionMeta, WorldDeltaData, WorldProjection, WorldSnapshotData } from "../session/protocol";

export type UnitView = {
  id: number;
  /** Row-major tile index. */
  tile: number;
  owner: number;
  health: number;
};

export type SettlementView = {
  id: number;
  tile: number;
  owner: number;
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
  /** In meters, zeroed on tiles the viewer has never explored. */
  elevations: Int16Array;
  /** `TileVisibility` values. */
  visibility: Uint8Array;
  tileOwner: Uint16Array;
  tileFlags: Uint8Array;
  units: Map<number, UnitView>;
  settlements: Map<number, SettlementView>;
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
});

const fromSnapshot = (snapshot: WorldSnapshotData): WorldState => {
  const units = new Map<number, UnitView>();
  for (let index = 0; index < snapshot.units.ids.length; index++) {
    const id = snapshot.units.ids[index];
    units.set(id, {
      id,
      tile: snapshot.units.tiles[index],
      owner: snapshot.units.owners[index],
      health: snapshot.units.health[index],
    });
  }
  const settlements = new Map<number, SettlementView>();
  for (let index = 0; index < snapshot.settlements.ids.length; index++) {
    const id = snapshot.settlements.ids[index];
    settlements.set(id, {
      id,
      tile: snapshot.settlements.tiles[index],
      owner: snapshot.settlements.owners[index],
    });
  }
  return {
    meta: snapshot.meta,
    columns: snapshot.columns,
    rows: snapshot.rows,
    terrain: snapshot.terrain,
    elevations: snapshot.elevations,
    visibility: snapshot.visibility,
    tileOwner: snapshot.tileOwner,
    tileFlags: snapshot.tileFlags,
    units,
    settlements,
  };
};

const patch = (state: WorldState, delta: WorldDeltaData): WorldState => {
  // the tile arrays of a delta are parallel to its indices, not to the map
  for (let index = 0; index < delta.tileIndices.length; index++) {
    const tile = delta.tileIndices[index];
    state.terrain[tile] = delta.terrain[index];
    state.elevations[tile] = delta.elevations[index];
    state.visibility[tile] = delta.visibility[index];
    state.tileOwner[tile] = delta.tileOwner[index];
    state.tileFlags[tile] = delta.tileFlags[index];
  }

  for (const id of delta.unitsRemoved) {
    state.units.delete(id);
  }
  for (let index = 0; index < delta.units.ids.length; index++) {
    const id = delta.units.ids[index];
    state.units.set(id, {
      id,
      tile: delta.units.tiles[index],
      owner: delta.units.owners[index],
      health: delta.units.health[index],
    });
  }

  for (const id of delta.settlementsRemoved) {
    state.settlements.delete(id);
  }
  for (let index = 0; index < delta.settlements.ids.length; index++) {
    const id = delta.settlements.ids[index];
    state.settlements.set(id, {
      id,
      tile: delta.settlements.tiles[index],
      owner: delta.settlements.owners[index],
    });
  }

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
