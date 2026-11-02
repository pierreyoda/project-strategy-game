import { BUILDING_TYPE_NAMES, SETTLEMENT_STAGE_NAMES, type SettlementStage } from "../../map/entities";
import type { OffsetCoords } from "../../map/hexMap";
import { TerrainKind, isExplored } from "../../map/terrain";
import { groundName, isForest } from "../../map/vegetation";
import { nationColor } from "../nations";
import type { Resource } from "../resources";
import { BuildingState, NO_NATION, type ResourceAmounts, TileFlags, TileVisibility } from "../session/protocol";
import type { WorldState } from "./worldState";

/** A nation, as a tile panel names and colours it. */
export type TileNation = { nation: number; name: string; color: number };

/**
 * A building as the panels show it: its type's name rather than the template's, the sprite it is
 * drawn with, how many levels its template has - for the level pips - and its works and upkeep.
 */
export type TileBuilding = {
  id: number;
  /** Its template's ID, what an order names it by. */
  template: string | null;
  type: string | null;
  name: string;
  /** 0 while it is first being built. */
  level: number;
  sprite: string | null;
  levels: number;
  /** `BuildingState` bits. */
  state: number;
  /** Its works' turns: zero without works, and for a rival's. */
  turnsLeft: number;
  turnsTotal: number;
};

/**
 * Everything a tile panel shows of a tile, gathered from the world's projection.
 *
 * The optional rows are what the panels are drawn for but the projection does not carry yet: a
 * deposit's quantity, a tile's movement cost and defense, its yields. The game leaves them out and
 * the panels skip what is missing; the stories fill them in.
 */
export type TileDetails = {
  /** Row-major tile index. */
  index: number;
  coords: OffsetCoords;
  /** `null` while the tile is unexplored. */
  terrain: TerrainKind | null;
  /** The terrain's name, or the forest's over it: "Forest", "Forested hills". */
  terrainName: string;
  /** Whether a forest grows on the tile, as far as the viewer knows. */
  forest?: boolean;
  /** In meters, as last seen. */
  elevation: number;
  visibility: TileVisibility;
  owner: TileNation | null;
  /** The nation whose sphere of influence the tile lies in, when it belongs to none. */
  sphere: TileNation | null;
  /** The stage of the settlement on the tile, its name until settlements have names of their own. */
  settlement: string | null;
  supplyNode: boolean;
  infrastructure: boolean;
  hasDeposit: boolean;
  building: TileBuilding | null;
  movementCost?: number;
  /** As a share: 0.25 is +25%. */
  defense?: number;
  deposit?: TileDeposit;
  yields?: Partial<Record<Resource, number>>;
};

/** A deposit as the tile panels show it. */
export type TileDeposit = {
  resource: Resource;
  /** The stock left, 0 for a deposit that springs back instead. */
  left: number;
  /** What it recharges per turn, 0 for a stock: water's figure in place of a stock. */
  recharge?: number;
  /** 1 to 3, the simulation's reading of what is left. */
  level?: number;
  turnsLeft: number | null;
};

/** The tile's building slot, as the panels draw it. */
export type SlotView =
  | { state: "free" }
  | { state: "construction"; building: TileBuilding; turnsLeft: number; turnsTotal: number }
  | { state: "built"; building: TileBuilding }
  | { state: "foreign"; owner: TileNation; building: TileBuilding | null }
  | { state: "none"; reason: string }
  | { state: "settlement"; settlement: string };

/** A template a free slot can take, and what its first level asks. */
export type SlotOption = {
  /** What `constructBuilding` names it by. */
  template: string;
  type: string;
  /** Its first level's, for its portrait. */
  sprite: string | null;
  cost?: ResourceAmounts;
  turns?: number;
  /** Why it suits the tile, as a badge: an extractor on a deposit of what it extracts. */
  favoured?: string;
};

/** What a slot's actions cost and give, read from its templates. */
export type SlotFigures = {
  options?: SlotOption[];
  /** Per turn, at the level the building works at. */
  output?: ResourceAmounts;
  upkeep?: ResourceAmounts;
  /** The next level's, left out at the last. */
  upgrade?: { cost: ResourceAmounts; turns: number };
  /** What the works under way were paid. */
  spent?: ResourceAmounts;
  /** What cancelling them would give back. Nothing cancels in the simulation yet. */
  refund?: ResourceAmounts;
  /** The turn at the end of which the works under way complete. */
  endsTurn?: number;
};

const WATER: readonly TerrainKind[] = [TerrainKind.DeepWater, TerrainKind.ShallowWater];

const nationOf = (world: WorldState, nation: number): TileNation | null =>
  nation === NO_NATION
    ? null
    : {
        nation,
        name: world.meta.nations.find(({ id }) => id === nation)?.name ?? `Nation ${nation + 1}`,
        color: nationColor(nation),
      };

export const buildingName = (type: string | null): string =>
  type === null ? "Unknown building" : (BUILDING_TYPE_NAMES[type] ?? type);

/** Gather what the projection says of the tile at `index`. */
export const tileDetailsOf = (world: WorldState, index: number): TileDetails => {
  const terrain = world.terrain[index];
  const forest = isForest(world.vegetation[index]);
  const flags = world.tileFlags[index];
  const building = [...world.buildings.values()].find(({ tile }) => tile === index);
  // the richer one, on the rare tile a guarantee laid a second deposit on
  const deposit = [...world.deposits.values()]
    .filter(({ tile, resource }) => tile === index && resource !== null)
    .sort((first, second) => second.level - first.level)[0];
  const settlement = [...world.settlements.values()].find(({ tile }) => tile === index);
  return {
    index,
    coords: { col: index % world.columns, row: Math.floor(index / world.columns) },
    terrain: isExplored(terrain) ? terrain : null,
    terrainName: isExplored(terrain) ? groundName(terrain, forest) : "Unexplored",
    forest,
    elevation: world.elevations[index],
    visibility: world.visibility[index] as TileVisibility,
    owner: nationOf(world, world.tileOwner[index]),
    sphere: nationOf(world, world.tileSphere[index]),
    settlement: settlement ? SETTLEMENT_STAGE_NAMES[settlement.stage as SettlementStage] : null,
    supplyNode: (flags & TileFlags.SupplyNode) !== 0,
    infrastructure: (flags & TileFlags.Infrastructure) !== 0,
    hasDeposit: (flags & TileFlags.Deposit) !== 0,
    deposit:
      deposit && deposit.resource
        ? {
            resource: deposit.resource,
            left: deposit.quantity,
            recharge: deposit.recharge,
            level: deposit.level,
            turnsLeft: deposit.turnsLeft,
          }
        : undefined,
    building: building
      ? {
          id: building.id,
          template: building.template,
          type: building.type,
          name: buildingName(building.type),
          level: building.level,
          sprite: building.sprite,
          levels: building.levels,
          state: building.state,
          turnsLeft: building.turnsLeft,
          turnsTotal: building.turnsTotal,
        }
      : null,
  };
};

/**
 * The state of a tile's building slot, for the panels to draw.
 *
 * Presentation only: the simulation judges every order. What is here mirrors its rules - a nation
 * builds on its own land, never on water, one building to a tile - so that the panel only offers
 * what an order would be accepted for.
 */
export const slotOf = (details: TileDetails, viewer: number): SlotView => {
  if (details.terrain === null) {
    return { state: "none", reason: "Nothing is known of this tile yet." };
  }
  if (details.settlement) {
    return { state: "settlement", settlement: details.settlement };
  }
  if (details.owner && details.owner.nation !== viewer) {
    return { state: "foreign", owner: details.owner, building: details.building };
  }
  const building = details.building;
  if (building) {
    return (building.state & BuildingState.UnderConstruction) !== 0
      ? { state: "construction", building, turnsLeft: building.turnsLeft, turnsTotal: building.turnsTotal }
      : { state: "built", building };
  }
  if (WATER.includes(details.terrain)) {
    return { state: "none", reason: "Nothing can be built on water." };
  }
  if (!details.owner) {
    return { state: "none", reason: "Only the nation's own land can be built on." };
  }
  return { state: "free" };
};

/**
 * What a slot's actions cost and give, read from the templates the meta carries.
 *
 * A free slot offers every template the tile can take: an extractor only on a deposit of what it
 * extracts, with something left. A building shows its level's output and upkeep, and the next
 * level's cost; works under way, what they were paid and when they complete.
 */
export const slotFiguresOf = (world: WorldState, details: TileDetails, slot: SlotView): SlotFigures => {
  const templates = world.meta.buildingTemplates;
  if (slot.state === "free") {
    const deposit = details.deposit;
    const takes = (resource: Resource | null) =>
      resource === null || (deposit?.resource === resource && (deposit.level ?? 0) > 0);
    return {
      options: templates
        .filter(({ deposit }) => takes(deposit))
        .map(({ id, type, deposit, levels: [first] }) => ({
          template: id,
          type,
          sprite: first?.sprite ?? null,
          cost: first?.cost ?? {},
          turns: first?.turns ?? 0,
          favoured: deposit === null ? undefined : "Deposit",
        })),
    };
  }
  if (slot.state !== "construction" && slot.state !== "built") {
    return {};
  }

  const { building } = slot;
  const template = templates.find(({ id }) => id === building.template);
  if (!template) {
    return {};
  }
  // level n is `levels[n - 1]`: the one it works at, and the next one, which works target
  const current = template.levels[building.level - 1];
  const next = template.levels[building.level];
  const underWay = building.turnsTotal > 0;
  return {
    output: current?.production,
    upkeep: current?.maintenance,
    upgrade: next && { cost: next.cost, turns: next.turns },
    spent: underWay ? next?.cost : undefined,
    // its last turn is the one it reaches 1 on
    endsTurn: underWay ? world.meta.turn + building.turnsLeft - 1 : undefined,
  };
};
