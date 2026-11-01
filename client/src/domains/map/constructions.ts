/**
 * A building going up on a tile, and how long it has left to go.
 *
 * Plain data, like `MapEntity`: what a construction is and how long one takes are the simulation's to
 * say, and the map only draws what it is handed.
 */
export type MapConstruction = {
  id: number;
  /** Row-major tile index. */
  tile: number;
  /** Turns until it is finished, from 1: the turn it reaches 1 is its last. */
  turnsLeft: number;
  /** Turns the whole construction takes, for the border to show the share done. */
  turnsTotal: number;
};

/**
 * The share of a construction done, in [0, 1]: what every drawing of its progress shows, the map's
 * border as the tile panel's ring, so that the two never disagree.
 */
export const constructionProgress = ({
  turnsLeft,
  turnsTotal,
}: Pick<MapConstruction, "turnsLeft" | "turnsTotal">): number =>
  turnsTotal > 0 ? Math.min(Math.max((turnsTotal - turnsLeft) / turnsTotal, 0), 1) : 0;
