import { TerrainKind } from "../../../map/terrain";
import { nationColor } from "../../nations";
import { TileVisibility } from "../../session/protocol";
import type { TileDetails, TileNation } from "../../state/tileDetails";
import type { SlotFigures } from "./BuildingSlot.svelte";

/** Tiles for the stories, as the UI kit's Tile focus board draws them. */

export const YOU: TileNation = { nation: 0, name: "Landfall", color: nationColor(0) };
export const RIVAL: TileNation = { nation: 1, name: "Nation 2", color: nationColor(1) };

/** The board's tile: hills with a level 2 mine, every row the projection does not carry yet filled in. */
export const HILLS: TileDetails = {
  index: 4 * 40 + 8,
  coords: { col: 17, row: 10 },
  terrain: TerrainKind.Hills,
  terrainName: "Hills",
  elevation: 412,
  visibility: TileVisibility.Visible,
  owner: YOU,
  sphere: YOU,
  settlement: null,
  supplyNode: true,
  infrastructure: true,
  hasDeposit: true,
  building: { type: "mine", name: "Mine", level: 2 },
  movementCost: 2,
  defense: 0.25,
  deposit: { resource: "metals", left: 340, turnsLeft: 113 },
  yields: { metals: 3 },
};

/** What the game itself can tell of a tile today: no figures beyond the projection's. */
export const SPARSE: TileDetails = {
  index: 12,
  coords: { col: 12, row: 0 },
  terrain: TerrainKind.Flat,
  terrainName: "Flat",
  elevation: 86,
  visibility: TileVisibility.Fogged,
  owner: null,
  sphere: YOU,
  settlement: null,
  supplyNode: false,
  infrastructure: false,
  hasDeposit: false,
  building: null,
};

export const FIGURES: SlotFigures = {
  options: [
    { type: "mine", cost: 40, turns: 5, favoured: "Deposit" },
    { type: "solar_plant", cost: 60, turns: 4 },
  ],
  output: { resource: "metals", amount: 3 },
  upkeep: 2,
  upgrade: { cost: 80, turns: 4 },
  spent: 40,
  refund: 20,
  endsTurn: 44,
};
