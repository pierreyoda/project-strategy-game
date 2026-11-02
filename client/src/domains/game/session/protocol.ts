/**
 * The wire format between the client and the `wasm_bridge` crate, mirrored by hand.
 *
 * Mirrored rather than generated, like `TerrainKind` mirrors the `procgen` enum: each type below
 * names its Rust counterpart, and the two move together.
 *
 * Commands and events are JSON because they are low-frequency and shape-heavy. The map is not: it
 * travels as typed arrays, see `projection.ts` in `core`.
 */

import type { Resource } from "../resources";

/** Offset coordinates, like `OffsetCoordsWire` in the `wasm_bridge` crate. */
export type OffsetCoordsWire = {
  col: number;
  row: number;
};

/** How a new game starts, like `NewGameSettings` in the `wasm_bridge` crate. */
export type NewGameSettings = {
  seed: number;
  columns: number;
  rows: number;
  /** How many nations play, the local player included. Defaults to 2 in the bridge. */
  nations?: number;
  /** Share of the map covered by the seas, from 0 to 1. Defaults to 0.575 in the bridge. */
  oceanCoverage?: number;
  /**
   * The scripts the game runs: one scenario, which a game cannot start without, and at most one AI
   * script. For now the AI script plays every nation but the local player's - the stub does without
   * one - and nothing yet hands part of the player's nation to a script.
   */
  scripts: readonly ScriptSource[];
};

/** A script sent with a new game, like `ScriptSourceWire` in the `wasm_bridge` crate. */
export type ScriptSource = {
  /** What the script is called in errors and in the turn log. */
  name: string;
  /** Like `ScriptKind` in the `scripting` crate: decides the hooks the script must declare. */
  kind: "ai" | "scenario";
  source: string;
};

/** An order, like `GameCommand` in the `core` crate. */
export type GameCommand =
  | { type: "moveUnit"; unit: number; path: readonly OffsetCoordsWire[] }
  | { type: "foundSettlement"; founder: number; name: string }
  /** On a tile of the nation's own land; the building belongs to its nearest settlement. */
  | { type: "constructBuilding"; template: string; tile: OffsetCoordsWire }
  /** The building's next level, one at a time. */
  | { type: "upgradeBuilding"; building: number };

/** Why an order was refused, like `CommandRejection` in the `core` crate. */
export type CommandRejection = {
  kind:
    | "notYourTurn"
    | "unknownEntity"
    | "unknownDefinition"
    | "notYours"
    | "illegalMove"
    | "tileOccupied"
    | "insufficientResources"
    | "illegalPlacement"
    | "missingDeposit"
    | "noSettlement"
    | "atMaxLevel"
    | "worksUnderWay"
    | "notImplemented";
  message: string;
};

/** Something that happened during a turn, like `GameEvent` in the `core` crate. */
export type GameEvent =
  | { type: "turnStarted"; turn: number }
  | { type: "log"; nation: number | null; message: string }
  | { type: "unitMoved"; unit: number; from: number; to: number }
  | { type: "settlementFounded"; settlement: number; nation: number; tile: number }
  /*
   * The building events name the nation they are about: how its buildings fare is its own to know,
   * like `BuildingHalted` and the rest of them in the `core` crate.
   */
  | { type: "constructionStarted"; nation: number; building: number; settlement: number }
  | { type: "upgradeStarted"; nation: number; building: number; level: number }
  /** Its works completed: it works at `level` from the next turn on. */
  | { type: "buildingCompleted"; nation: number; building: number; level: number }
  /** It stopped for want of its maintenance; told the turn it stops, once. */
  | { type: "buildingHalted"; nation: number; building: number }
  /** A deposit the nation drew on ran dry. */
  | { type: "depositExhausted"; nation: number; deposit: number }
  | { type: "resourceShortage"; nation: number; resource: Resource }
  | { type: "commandRejected"; nation: number; reason: string }
  /** What a controller said while planning for a nation, like `ControllerLog`: not narrative. */
  | { type: "controllerLog"; nation: number; controller: string; message: string };

/** A phase of turn resolution, like `TurnPhase::name` in the `core` crate. */
export type TurnPhase =
  | "player_commit"
  | "ai_planning"
  | "economy"
  | "logistics"
  | "military"
  | "population"
  | "fog"
  | "finalize"
  | "idle"
  | "done";

export const TURN_PHASE_NAMES: Record<TurnPhase, string> = {
  player_commit: "Committing orders",
  ai_planning: "Other nations are planning",
  economy: "Economy",
  logistics: "Logistics",
  military: "Military",
  population: "Population",
  fog: "Reconnaissance",
  finalize: "Wrapping up the turn",
  idle: "Idle",
  done: "Done",
};

/** What a nation knows of a tile, like `TileVisibility` in the `core` crate. */
export const TileVisibility = {
  Unexplored: 0,
  Fogged: 1,
  Visible: 2,
} as const;
export type TileVisibility = (typeof TileVisibility)[keyof typeof TileVisibility];

/** Bits of the per-tile flags buffer, like `projection::tile_flags` in the `core` crate. */
export const TileFlags = {
  Settlement: 1 << 0,
  Building: 1 << 1,
  SupplyNode: 1 << 2,
  Infrastructure: 1 << 3,
  Deposit: 1 << 4,
} as const;

/** How far a settlement has grown, like `SettlementStage` in the `core` crate. */
export const SettlementStage = {
  Outpost: 0,
  Colony: 1,
  Town: 2,
  City: 3,
  Capital: 4,
} as const;
export type SettlementStage = (typeof SettlementStage)[keyof typeof SettlementStage];

/** The "no nation" value of the owner buffers, like `projection::NO_NATION` in the `core` crate. */
export const NO_NATION = 0xffff;

/** A nation's stockpile of one resource, like `ResourceWire` in the `wasm_bridge` crate. */
export type ResourceMeta = {
  resource: Resource;
  quantity: number;
  income: number | null;
  depletion: number | null;
};

/** A nation, like `NationWire` in the `wasm_bridge` crate. */
export type NationMeta = {
  id: number;
  name: string;
  /** Only ever filled for the viewer: what a rival holds is not public knowledge. */
  resources: ResourceMeta[];
};

/** The small half of a projection, like `MetaWire` in the `wasm_bridge` crate. */
export type ProjectionMeta = {
  version: number;
  turn: number;
  viewer: number;
  activeNation: number;
  nations: NationMeta[];
  /** What `BuildingsBuffers.kinds` index into, like `BuildingTemplateWire` in the `wasm_bridge` crate. */
  buildingTemplates: BuildingTemplateMeta[];
  /** What `UnitsBuffers.kinds` index into, like `UnitTemplateWire` in the `wasm_bridge` crate. */
  unitTemplates: UnitTemplateMeta[];
};

/** Quantities of resources, by name: a cost, an upkeep, an output. */
export type ResourceAmounts = Partial<Record<Resource, number>>;

/** A level of a building template, like `BuildingLevelWire` in the `wasm_bridge` crate. */
export type BuildingLevelMeta = {
  /** What the level is drawn with: a frame of the `buildings` sheet, `{sprite}.png`. */
  sprite: string;
  /** What reaching the level costs: the construction for the first, an upgrade above. */
  cost: ResourceAmounts;
  /** Turns the works to reach it take. */
  turns: number;
  /** Per turn, while it works at this level. */
  maintenance: ResourceAmounts;
  production: ResourceAmounts;
};

export type BuildingTemplateMeta = {
  /** What `constructBuilding` names it by: a scenario may give it the type of another. */
  id: string;
  type: string;
  /** The resource it extracts from the deposit it stands on, `null` for none. */
  deposit: Resource | null;
  /** Level n is `levels[n - 1]`. */
  levels: BuildingLevelMeta[];
};

export type UnitTemplateMeta = {
  type: string;
  /** Movement points per turn. */
  movement: number;
  sight: number;
};

/** The entities of a projection, as parallel arrays. */
export type UnitsBuffers = {
  ids: Uint32Array;
  tiles: Uint32Array;
  owners: Uint16Array;
  health: Uint16Array;
  /** Indices into `ProjectionMeta.unitTemplates`, `UNKNOWN_UNIT_KIND` for a template nobody registered. */
  kinds: Uint8Array;
  /** Movement points left this turn. */
  movementLeft: Uint8Array;
};

/** A unit whose template is unknown, like `projection::UNKNOWN_UNIT_KIND` in the `core` crate. */
export const UNKNOWN_UNIT_KIND = 0xff;

export type SettlementsBuffers = {
  ids: Uint32Array;
  tiles: Uint32Array;
  owners: Uint16Array;
  /** `SettlementStage` values. */
  stages: Uint8Array;
};

export type BuildingsBuffers = {
  ids: Uint32Array;
  tiles: Uint32Array;
  owners: Uint16Array;
  /** Indices into `ProjectionMeta.buildingTemplates`, `UNKNOWN_BUILDING_KIND` for a template nobody registered. */
  kinds: Uint8Array;
  /** 0 while a building is first being built. */
  levels: Uint8Array;
  /** `BuildingState` bits: its works, and for the viewer's own whether it is halted. */
  states: Uint8Array;
  /** Turns its works have left, and take in all: zero without works, and for a rival's. */
  turnsLeft: Uint16Array;
  turnsTotal: Uint16Array;
};

/** Bits of `BuildingsBuffers.states`, like `projection::building_states` in the `core` crate. */
export const BuildingState = {
  /** Its first construction is under way: level 0, working nothing yet. */
  UnderConstruction: 1 << 0,
  /** An upgrade is under way: it works at its level meanwhile. */
  Upgrading: 1 << 1,
  /** Its upkeep went unpaid: it makes nothing. Never set for a rival's. */
  Halted: 1 << 2,
} as const;

/** A building whose template is unknown, like `projection::UNKNOWN_BUILDING_KIND` in the `core` crate. */
export const UNKNOWN_BUILDING_KIND = 0xff;

/** The deposits the viewer knows of, like `projection::DepositsBuffers` in the `core` crate. */
export type DepositsBuffers = {
  ids: Uint32Array;
  tiles: Uint32Array;
  /** Indices into `RESOURCES`, `UNKNOWN_RESOURCE` for a deposit holding nothing. */
  resources: Uint8Array;
  /** 1 to `DEPOSIT_LEVELS`, 0 once exhausted. */
  levels: Uint8Array;
  /** The stock left, 0 for a deposit that springs back instead. */
  quantities: Uint16Array;
  /** What it recharges per turn, 0 for a stock. */
  recharges: Uint16Array;
  /** Turns its stock lasts at the rate it is drawn from, `NO_TURNS_LEFT` while nothing draws on it. */
  turnsLeft: Uint16Array;
};

/** Like `projection::UNKNOWN_RESOURCE` in the `core` crate. */
export const UNKNOWN_RESOURCE = 0xff;
/** Like `projection::NO_TURNS_LEFT` in the `core` crate. */
export const NO_TURNS_LEFT = 0xffff;
/** Like `resources::DEPOSIT_LEVELS` in the `core` crate: trace, deposit, rich. */
export const DEPOSIT_LEVELS = 3;

/** Everything the viewer can see, like `WorldSnapshot` in the `core` crate. */
export type WorldSnapshotData = {
  meta: ProjectionMeta;
  columns: number;
  rows: number;
  /** `TerrainKind` values, `Unknown` wherever the viewer has never explored. */
  terrain: Uint8Array;
  /** `Vegetation` values, `UNKNOWN_VEGETATION` wherever the viewer has never explored. */
  vegetation: Uint8Array;
  /** In meters, zeroed on tiles the viewer has never explored. */
  elevations: Int16Array;
  /** `TileVisibility` values. */
  visibility: Uint8Array;
  tileOwner: Uint16Array;
  /**
   * Nation whose sphere of influence holds each tile, `NO_NATION` where none or unknown: the
   * viewer's own everywhere, a rival's only once the tile is explored.
   */
  tileSphere: Uint16Array;
  tileFlags: Uint8Array;
  units: UnitsBuffers;
  settlements: SettlementsBuffers;
  buildings: BuildingsBuffers;
  deposits: DepositsBuffers;
};

/**
 * What changed for the viewer, like `WorldDelta` in the `core` crate.
 *
 * The tile arrays are parallel to `tileIndices`, not to the map.
 */
export type WorldDeltaData = {
  meta: ProjectionMeta;
  /** Row-major indices of the tiles that changed, in ascending order. */
  tileIndices: Uint32Array;
  /** `TerrainKind` values of those tiles: a tile explored this turn stops being `Unknown`. */
  terrain: Uint8Array;
  /** `Vegetation` values of those tiles, like `terrain`. */
  vegetation: Uint8Array;
  elevations: Int16Array;
  visibility: Uint8Array;
  tileOwner: Uint16Array;
  /**
   * Nation whose sphere of influence holds each tile, `NO_NATION` where none or unknown: the
   * viewer's own everywhere, a rival's only once the tile is explored.
   */
  tileSphere: Uint16Array;
  tileFlags: Uint8Array;
  units: UnitsBuffers;
  /** Units to drop: destroyed, or simply no longer in sight. */
  unitsRemoved: Uint32Array;
  settlements: SettlementsBuffers;
  settlementsRemoved: Uint32Array;
  buildings: BuildingsBuffers;
  buildingsRemoved: Uint32Array;
  /** Deposits on the changed tiles the viewer knows, those just explored mostly. None is ever removed. */
  deposits: DepositsBuffers;
};

/**
 * The tiles a unit can end its turn on, like `ReachableView` in the `wasm_bridge` crate: three
 * parallel arrays, `tiles` ascending and the unit's own tile among them, `parents` naming the tile
 * before each one on its cheapest path (the origin names itself). Built for the viewer, so it never
 * mentions a tile the viewer has not explored.
 */
export type ReachableData = {
  tiles: Uint32Array;
  costs: Uint8Array;
  parents: Uint32Array;
};

/**
 * Either a catch-up or a whole new snapshot.
 *
 * Which one it is is the simulation's call, not the client's: there is one rule for it, in `core`.
 */
export type WorldProjection = { kind: "full"; snapshot: WorldSnapshotData } | { kind: "delta"; delta: WorldDeltaData };

/** Messages from the main thread to `gameSession.worker.ts`. */
export type GameSessionRequest =
  | { type: "start"; id: number; settings: NewGameSettings }
  | { type: "submit"; id: number; command: GameCommand }
  | { type: "endTurn"; id: number }
  | { type: "snapshot"; id: number; viewer: number }
  | { type: "project"; id: number; viewer: number; since: number }
  /** Where the viewer's unit can get to this turn. */
  | { type: "reachable"; id: number; viewer: number; unit: number }
  /** Stop the turn being resolved, between two steps. Never by terminating the worker. */
  | { type: "cancelTurn" };

/** Messages from `gameSession.worker.ts` to the main thread. */
export type GameSessionResponse =
  | { type: "ready"; id: number; localPlayer: number; turn: number; version: number }
  | { type: "accepted"; id: number; version: number }
  | { type: "rejected"; id: number; rejection: CommandRejection }
  | { type: "progress"; phase: TurnPhase; percent: number }
  | { type: "events"; events: GameEvent[] }
  | { type: "turnComplete"; id: number; turn: number; version: number }
  | { type: "turnCancelled"; id: number }
  | { type: "projection"; id: number; projection: WorldProjection }
  /** `null` for a unit the viewer does not own. */
  | { type: "reachable"; id: number; reach: ReachableData | null }
  | { type: "error"; id: number | null; message: string };
