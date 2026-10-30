import type { HexMapData, OffsetCoords } from "../../map/hexMap";
import { MAX_SEED } from "../../map/procgen/planetGenerator";
import { RESOURCES, type Resource, type ResourceStock } from "../resources";
import { type GameSessionHandle, createGameSession } from "../session/gameSession";
import type {
  CommandRejection,
  GameEvent,
  NewGameSettings,
  ProjectionMeta,
  TurnPhase,
  WorldProjection,
} from "../session/protocol";
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
  /** The nation being played, whose eyes everything is seen through. */
  readonly viewer: number;
  readonly turn: number;
  /** The viewer's own stockpiles, every resource present even when it holds none. */
  readonly stocks: Record<Resource, ResourceStock>;
  readonly selected: SelectedUnit | null;
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
  let viewer = $state(0);

  let phase = $state<TurnPhase | null>(null);
  let percent = $state(0);
  let resolving = $state(false);
  let starting = $state(false);
  let error = $state<string | null>(null);
  let log = $state<GameLogEntry[]>([]);
  let selectedUnit = $state<number | null>(null);
  let focusTile = $state<number | null>(null);

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
  };

  /** Catch up with everything that changed since the projection currently drawn. */
  const catchUp = async (since: number) => apply(await session.project(viewer, since));

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
    selectedUnit = null;
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
      if (selectedUnit === null || !world) {
        return null;
      }
      const unit = world.units.get(selectedUnit);
      return unit ? { id: unit.id, tile: unit.tile } : null;
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
        return;
      }
      if (selectedUnit === null) {
        return;
      }
      // no rule is checked here: whether a step is legal is the simulation's to say
      const rejection = await session.submit({
        type: "moveUnit",
        unit: selectedUnit,
        path: [{ col: tile.col, row: tile.row }],
      });
      if (rejection) {
        record([{ kind: "rejected", rejection }]);
        return;
      }
      await catchUp(meta.version);
    },

    dispose: () => {
      unsubscribe();
      session.dispose();
    },
  };
};
