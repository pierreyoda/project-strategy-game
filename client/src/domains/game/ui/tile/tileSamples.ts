import { TerrainKind } from "../../../map/terrain";
import { nationColor } from "../../nations";
import { TileVisibility } from "../../session/protocol";
import type { SlotFigures, TileDetails, TileNation } from "../../state/tileDetails";

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
  district: null,
  building: {
    id: 0,
    template: "mine",
    type: "mine",
    name: "Mine",
    level: 2,
    sprite: "mine_2",
    levels: 3,
    state: 0,
    turnsLeft: 0,
    turnsTotal: 0,
  },
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
  district: null,
  building: null,
};

export const FIGURES: SlotFigures = {
  options: [
    {
      template: "mine",
      type: "mine",
      sprite: "mine_1",
      cost: { credits: 200, metals: 100 },
      turns: 3,
      favoured: "Deposit",
    },
    {
      template: "solar_plant",
      type: "solar_plant",
      sprite: "solar_plant_1",
      cost: { credits: 500, metals: 250 },
      turns: 4,
    },
  ],
  output: { metals: 8 },
  upkeep: { credits: 3 },
  upgrade: { cost: { credits: 450, metals: 250 }, turns: 5 },
  spent: { credits: 200, metals: 100 },
  refund: { credits: 100, metals: 50 },
  endsTurn: 44,
};
