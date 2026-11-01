import { BUILDING_TYPE_NAMES, SETTLEMENT_STAGE_NAMES, type SettlementStage } from "../../map/entities";
import type { OffsetCoords } from "../../map/hexMap";
import { TERRAIN_NAMES, TerrainKind, isExplored } from "../../map/terrain";
import { nationColor } from "../nations";
import type { Resource } from "../resources";
import { NO_NATION, TileFlags, TileVisibility } from "../session/protocol";
import type { WorldState } from "./worldState";

/** A nation, as a tile panel names and colours it. */
export type TileNation = { nation: number; name: string; color: number };

/** A building as the panels show it: its type's name rather than the template's. */
export type TileBuilding = { type: string | null; name: string; level: number };

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
  terrainName: string;
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
  deposit?: { resource: Resource; left: number; turnsLeft: number | null };
  yields?: Partial<Record<Resource, number>>;
};

/**
 * The tile's building slot, as the panels draw it.
 *
 * `construction` only ever comes from the stories until the simulation has construction time.
 */
export type SlotView =
  | { state: "free" }
  | { state: "construction"; building: TileBuilding; turnsLeft: number; turnsTotal: number }
  | { state: "built"; building: TileBuilding }
  | { state: "foreign"; owner: TileNation; building: TileBuilding | null }
  | { state: "none"; reason: string }
  | { state: "settlement"; settlement: string };

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
  type !== null && type in BUILDING_TYPE_NAMES
    ? BUILDING_TYPE_NAMES[type as keyof typeof BUILDING_TYPE_NAMES]
    : (type ?? "Unknown building");

/** Gather what the projection says of the tile at `index`. */
export const tileDetailsOf = (world: WorldState, index: number): TileDetails => {
  const terrain = world.terrain[index];
  const flags = world.tileFlags[index];
  const building = [...world.buildings.values()].find(({ tile }) => tile === index);
  const settlement = [...world.settlements.values()].find(({ tile }) => tile === index);
  return {
    index,
    coords: { col: index % world.columns, row: Math.floor(index / world.columns) },
    terrain: isExplored(terrain) ? terrain : null,
    terrainName: isExplored(terrain) ? TERRAIN_NAMES[terrain] : "Unexplored",
    elevation: world.elevations[index],
    visibility: world.visibility[index] as TileVisibility,
    owner: nationOf(world, world.tileOwner[index]),
    sphere: nationOf(world, world.tileSphere[index]),
    settlement: settlement ? SETTLEMENT_STAGE_NAMES[settlement.stage as SettlementStage] : null,
    supplyNode: (flags & TileFlags.SupplyNode) !== 0,
    infrastructure: (flags & TileFlags.Infrastructure) !== 0,
    hasDeposit: (flags & TileFlags.Deposit) !== 0,
    building: building ? { type: building.type, name: buildingName(building.type), level: building.level } : null,
  };
};

/**
 * The state of a tile's building slot, for the panels to draw.
 *
 * Presentation only: no game rule is decided here. Which terrain may hold a building and who may
 * build where are the simulation's to say once it has building slots; until then this is what the
 * panels assume, in one place to replace.
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
  if (details.building) {
    return { state: "built", building: details.building };
  }
  if (WATER.includes(details.terrain)) {
    return { state: "none", reason: "Nothing can be built on water." };
  }
  return { state: "free" };
};
