import type { MapEntity } from "../../map/entities";
import type { HexMapData, OffsetCoords } from "../../map/hexMap";
import { MAX_SEED } from "../../map/procgen/planetGenerator";
import { RESOURCES, type Resource, type ResourceStock } from "../resources";
import { type GameSessionHandle, createGameSession } from "../session/gameSession";
import type {
  CommandRejection,
  GameEvent,
  NewGameSettings,
  ProjectionMeta,
  ReachableData,
  TurnPhase,
  WorldProjection,
} from "../session/protocol";
import { mapEntitiesOf } from "./mapEntities";
import { type WorldState, applyProjection, toHexMapData } from "./worldState";

/** How many entries of the turn log are kept. */
const LOG_LENGTH = 40;

/** The map a new game starts on. TODO: until there is a new-game screen to choose it. */
const DEFAULT_MAP = { columns: 60, rows: 40, nations: 3 };

/**
 * Something worth a line in the turn log, before anything decides how to word it.
 *
 * Kept raw on purpose: what the player reads is the view's business, and wording that lives next to
 * the logic is wording nobody finds when it needs changing.
 */
export type GameLogEntry =
  { kind: "event"; event: GameEvent } | { kind: "rejected"; rejection: CommandRejection } | { kind: "turnCancelled" };

/** The viewer's unit the player is giving orders to. */
export type SelectedUnit = { id: number; tile: number };

/**
 * The steps from the unit's tile to `tile`, destination last and origin left out - what a move
 * order takes - or `null` when the tile is out of reach. `tiles` is sorted, so the lookup is a
 * binary search and the walk follows `parents` back to the origin.
 */
export const pathTo = (reach: ReachableData, tile: number): number[] | null => {
  const position = (wanted: number): number | null => {
    let low = 0;
    let high = reach.tiles.length - 1;
    while (low <= high) {
      const middle = (low + high) >> 1;
      const found = reach.tiles[middle];
      if (found === wanted) {
        return middle;
      }
      if (found < wanted) {
        low = middle + 1;
      } else {
        high = middle - 1;
      }
    }
    return null;
  };
  let at = position(tile);
  if (at === null) {
    return null;
  }
  const path: number[] = [];
  while (reach.parents[at] !== reach.tiles[at]) {
    path.push(reach.tiles[at]);
    const parent = position(reach.parents[at]);
    if (parent === null) {
      return null;
    }
    at = parent;
  }
  return path.reverse();
};

/**
 * The game being played, as the client holds it.
 *
 * It owns the session worker and everything derived from what the simulation sends back. It owns no
 * rule: whether an order is legal is the simulation's to say, and a second opinion here is how the
 * two start to disagree.
 */
export type ActiveGame = {
  /** The projection the renderers draw, or `null` before the first one arrives. */
  readonly world: WorldState | null;
  /** The map the terrain renderer draws. A new object only when a full projection replaces it. */
  readonly map: HexMapData | null;
  /** Row-major indices of the tiles the last projection changed, `null` to repaint everything. */
  readonly dirtyTiles: Uint32Array | null;
  /**
   * The settlements the map renderer draws, rebuilt whole on every projection.
   *
   * Not derived from `world`: a delta patches its buffers in place, so nothing watching its
   * identity would ever re-run.
   */
  readonly entities: readonly MapEntity[];
  /** The nation being played, whose eyes everything is seen through. */
  readonly viewer: number;
  readonly turn: number;
  /** The viewer's own stockpiles, every resource present even when it holds none. */
  readonly stocks: Record<Resource, ResourceStock>;
  readonly selected: SelectedUnit | null;
  /**
   * Where the selected unit can get to this turn, as the simulation answered, or `null` while
   * nothing is selected or the answer has not come back. Drawn over the map, never decided here.
   */
  readonly reach: ReachableData | null;
  /** The steps to the hovered tile when it is in reach, for the map to preview; `null` otherwise. */
  readonly previewPath: readonly number[] | null;
  /** A tile worth framing the camera on, such as the viewer's unit when a new game starts. */
  readonly focusTile: number | null;
  readonly phase: TurnPhase | null;
  readonly percent: number;
  readonly resolving: boolean;
  readonly starting: boolean;
  readonly error: string | null;
  readonly log: readonly GameLogEntry[];

  /** Start a new game on a random seed. */
  startNewGame: () => Promise<void>;
  start: (settings: NewGameSettings) => Promise<void>;
  /** Resolve a turn, and catch up with everything it changed. */
  endTurn: () => Promise<void>;
  /** Stop the turn in progress between two phases. */
  cancelTurn: () => void;
  /** Select one of the viewer's units, or move the selected one onto the tile. */
  clickTile: (tile: OffsetCoords) => Promise<void>;
  /** The tile under the pointer, `null` once it leaves the map: what the path preview follows. */
  hoverTile: (tile: OffsetCoords | null) => void;
  /** Release the session worker. */
  dispose: () => void;
};

const randomSeed = () => Math.floor(Math.random() * (MAX_SEED + 1));

export const createActiveGame = (): ActiveGame => {
  const session: GameSessionHandle = createGameSession();

  let world = $state.raw<WorldState | null>(null);
  /**
   * The world's turn, nations and stockpiles, held apart from `world`.
   *
   * A delta patches the world's buffers in place, so its identity never changes and nothing reading
   * it would re-render. Every projection brings a fresh `meta` object, which is what the UI watches.
   */
  let meta = $state.raw<ProjectionMeta | null>(null);
  let map = $state.raw<HexMapData | null>(null);
  let dirtyTiles = $state.raw<Uint32Array | null>(null);
  let entities = $state.raw<readonly MapEntity[]>([]);
  let viewer = $state(0);

  let phase = $state<TurnPhase | null>(null);
  let percent = $state(0);
  let resolving = $state(false);
  let starting = $state(false);
  let error = $state<string | null>(null);
  let log = $state<GameLogEntry[]>([]);
  let selectedUnit = $state<number | null>(null);
  let focusTile = $state<number | null>(null);
  let reach = $state.raw<ReachableData | null>(null);
  let hoveredIndex = $state<number | null>(null);
  /** The latest reachability request: an older answer arriving late must not overwrite a newer one. */
  let reachRequest = 0;

  const record = (entries: GameLogEntry[]) => {
    if (entries.length > 0) {
      log = [...log, ...entries].slice(-LOG_LENGTH);
    }
  };

  const fail = (thrown: unknown) => {
    error = thrown instanceof Error ? thrown.message : String(thrown);
  };

  /**
   * Fold a projection into what is drawn.
   *
   * A full one replaces the map and the world, so both renderers draw themselves anew; a delta
   * patches their buffers in place and hands them the tiles to repaint.
   */
  const apply = (projection: WorldProjection) => {
    const applied = applyProjection(world, projection);
    if (projection.kind === "full") {
      map = toHexMapData(applied.state);
      // the viewer's own unit: at turn 1 it is the only thing there is to look at
      const own = [...applied.state.units.values()].find(({ owner }) => owner === viewer);
      focusTile = own?.tile ?? null;
    }
    world = applied.state;
    // a new object on every projection, unlike the world it belongs to
    meta = applied.state.meta;
    dirtyTiles = applied.dirtyTiles;
    // settlements are sparse: rebuilding the list costs less than tracking which ones moved
    entities = mapEntitiesOf(applied.state);
  };

  /**
   * Ask the simulation where the selected unit can get to. Asked again after every order and every
   * projection applied, since both can move the answer.
   */
  const refreshReach = async () => {
    const request = ++reachRequest;
    const unit = selectedUnit;
    if (unit === null || !world) {
      reach = null;
      return;
    }
    const answer = await session.reachable(viewer, unit);
    if (request === reachRequest) {
      reach = answer;
    }
  };

  /** Catch up with everything that changed since the projection currently drawn. */
  const catchUp = async (since: number) => {
    apply(await session.project(viewer, since));
    await refreshReach();
  };

  const unsubscribe = session.subscribe((notification) => {
    switch (notification.type) {
      case "progress":
        phase = notification.phase;
        percent = notification.percent;
        break;
      case "events":
        record(notification.events.map((event) => ({ kind: "event", event })));
        break;
      case "error":
        error = notification.message;
        break;
    }
  });

  const start = async (settings: NewGameSettings) => {
    starting = true;
    error = null;
    log = [];
    world = null;
    meta = null;
    map = null;
    dirtyTiles = null;
    entities = [];
    selectedUnit = null;
    reach = null;
    hoveredIndex = null;
    focusTile = null;
    try {
      const ready = await session.start(settings);
      viewer = ready.localPlayer;
      apply(await session.snapshot(ready.localPlayer));
    } catch (startError) {
      fail(startError);
    } finally {
      starting = false;
    }
  };

  return {
    get world() {
      return world;
    },
    get map() {
      return map;
    },
    get dirtyTiles() {
      return dirtyTiles;
    },
    get entities() {
      return entities;
    },
    get viewer() {
      return viewer;
    },
    get turn() {
      return meta?.turn ?? 0;
    },
    get stocks() {
      const stocks = Object.fromEntries(
        RESOURCES.map((resource) => [resource, { quantity: 0, income: 0, depletion: 0 }]),
      ) as Record<Resource, ResourceStock>;
      const nation = meta?.nations.find(({ id }) => id === viewer);
      for (const stock of nation?.resources ?? []) {
        stocks[stock.resource] = {
          quantity: stock.quantity,
          income: stock.income ?? 0,
          depletion: stock.depletion ?? 0,
        };
      }
      return stocks;
    },
    get selected() {
      // a delta patches the world in place, so the projection's fresh `meta` is what says the unit
      // may have moved since this was last read
      if (selectedUnit === null || !world || !meta) {
        return null;
      }
      const unit = world.units.get(selectedUnit);
      return unit ? { id: unit.id, tile: unit.tile } : null;
    },
    get reach() {
      return reach;
    },
    get previewPath() {
      if (hoveredIndex === null || !reach || selectedUnit === null) {
        return null;
      }
      const path = pathTo(reach, hoveredIndex);
      return path && path.length > 0 ? path : null;
    },
    get focusTile() {
      return focusTile;
    },
    get phase() {
      return phase;
    },
    get percent() {
      return percent;
    },
    get resolving() {
      return resolving;
    },
    get starting() {
      return starting;
    },
    get error() {
      return error;
    },
    get log() {
      return log;
    },

    start,
    startNewGame: () => start({ seed: randomSeed(), ...DEFAULT_MAP }),

    endTurn: async () => {
      if (!meta || resolving) {
        return;
      }
      resolving = true;
      error = null;
      try {
        const outcome = await session.endTurn();
        // the version the client last drew: everything since is what it has to catch up with
        await catchUp(meta.version);
        if (outcome.type === "cancelled") {
          record([{ kind: "turnCancelled" }]);
        }
      } catch (turnError) {
        fail(turnError);
      } finally {
        resolving = false;
        phase = null;
        percent = 0;
      }
    },

    cancelTurn: () => session.cancelTurn(),

    clickTile: async (tile: OffsetCoords) => {
      if (!world || !meta || resolving) {
        return;
      }
      const index = tile.row * world.columns + tile.col;
      const own = [...world.units.values()].find((unit) => unit.tile === index && unit.owner === viewer);
      if (own) {
        selectedUnit = own.id;
        await refreshReach();
        return;
      }
      if (selectedUnit === null) {
        return;
      }
      // the path the simulation itself offered, when it offered one; a tile it did not (never
      // explored, say) still gets the single step it always got, and the simulation's verdict on
      // it. No rule is checked here either way.
      const columns = world.columns;
      const steps = reach ? pathTo(reach, index) : null;
      const path =
        steps && steps.length > 0
          ? steps.map((step) => ({ col: step % columns, row: Math.floor(step / columns) }))
          : [{ col: tile.col, row: tile.row }];
      const rejection = await session.submit({
        type: "moveUnit",
        unit: selectedUnit,
        path,
      });
      if (rejection) {
        record([{ kind: "rejected", rejection }]);
        return;
      }
      await catchUp(meta.version);
    },

    hoverTile: (tile: OffsetCoords | null) => {
      hoveredIndex = tile && world ? tile.row * world.columns + tile.col : null;
    },

    dispose: () => {
      unsubscribe();
      session.dispose();
    },
  };
};
