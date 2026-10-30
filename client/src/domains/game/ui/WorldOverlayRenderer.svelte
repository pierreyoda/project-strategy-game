<script lang="ts" module>
  import { Container, Graphics, GraphicsContext } from "pixi.js";

  import type { PixiContext } from "../../../shared/pixi";
  import { type HexLayout, hexCenter, hexCorners, wrapBounds } from "../../map/hexLayout";
  import { nationColor } from "../nations";
  import { NO_NATION, type ReachableData, TileVisibility } from "../session/protocol";
  import type { WorldState } from "../state/worldState";

  /** The unseen is drawn in the background's own color rather than in black, which would look like a hole. */
  const FOG_COLOR = 0x05070c;
  /** Opacity of the fog over a tile that was explored but is not watched right now. */
  const FOGGED_ALPHA = 0.55;
  /** Opacity of the tint marking a tile's owner. */
  const OWNER_TINT_ALPHA = 0.2;
  /** Opacity of the tint marking a sphere of influence, over the fog where there is any. */
  const SPHERE_TINT_ALPHA = 0.1;
  /** The tiles the selected unit can reach: a tint and a thin edge, both in the viewer's colour. */
  const REACH_TINT_ALPHA = 0.28;
  const REACH_EDGE_ALPHA = 0.6;
  /** The path preview: a line through the hex centres, a dot on the destination. */
  const PATH_COLOR = 0xffffff;
  const PATH_ALPHA = 0.85;
  /** Drawn over the map, see `GameView`. */
  const OVERLAY_Z_INDEX = 10;

  /**
   * The drawn overlay, kept addressable row by row.
   *
   * What the viewer may see changes constantly while the terrain under it does not, so fog and borders
   * are their own layer: a turn repaints the rows its delta touched, and leaves the map alone.
   */
  type DrawnOverlay = {
    layer: Container;
    /** Fog and borders, one context per row, each drawn once per copy of the map. */
    rows: readonly GraphicsContext[];
    /** Units and the selection outline: few enough to redraw whole, rather than tracked one by one. */
    entities: GraphicsContext;
  };

  const paintRow = (drawn: DrawnOverlay, world: WorldState, layout: HexLayout, row: number) => {
    const context = drawn.rows[row].clear();
    const corners = hexCorners(layout);
    for (let col = 0; col < world.columns; col++) {
      const index = row * world.columns + col;
      const visibility = world.visibility[index];
      const owner = world.tileOwner[index];
      const sphere = world.tileSphere[index];
      const { x, y } = hexCenter(layout, { col, row });
      const hex = () => context.poly(corners.map((corner, i) => corner + (i % 2 === 0 ? x : y)));

      if (visibility !== TileVisibility.Unexplored && owner !== NO_NATION) {
        hex().fill({ color: nationColor(owner), alpha: OWNER_TINT_ALPHA });
      }
      if (visibility === TileVisibility.Unexplored) {
        hex().fill(FOG_COLOR);
      } else if (visibility === TileVisibility.Fogged) {
        hex().fill({ color: FOG_COLOR, alpha: FOGGED_ALPHA });
      }
      // over the fog: the viewer's own sphere reaches into what they have not explored yet, which
      // is the point of showing it
      if (sphere !== NO_NATION) {
        hex().fill({ color: nationColor(sphere), alpha: SPHERE_TINT_ALPHA });
      }
    }
  };

  /** What the player is doing with a unit: the selection, where it can go, where the pointer would send it. */
  type Orders = {
    selectedTile: number | null;
    reach: ReachableData | null;
    previewPath: readonly number[] | null;
    /** The viewer's nation, whose colour tints the reach. */
    viewer: number | null;
  };

  const paintEntities = (drawn: DrawnOverlay, world: WorldState, layout: HexLayout, orders: Orders) => {
    const context = drawn.entities.clear();
    const centerOf = (tile: number) =>
      hexCenter(layout, { col: tile % world.columns, row: Math.floor(tile / world.columns) });
    const hexAt = (tile: number) => {
      const { x, y } = centerOf(tile);
      return context.poly(hexCorners(layout).map((corner, i) => corner + (i % 2 === 0 ? x : y)));
    };

    // the reach, as the simulation answered it: a tint, nothing more, since the panel says the cost
    if (orders.reach && orders.viewer !== null) {
      for (const tile of orders.reach.tiles) {
        if (tile !== orders.selectedTile) {
          hexAt(tile)
            .fill({ color: nationColor(orders.viewer), alpha: REACH_TINT_ALPHA })
            .stroke({
              width: Math.max(layout.size * 0.05, 1),
              color: nationColor(orders.viewer),
              alpha: REACH_EDGE_ALPHA,
            });
        }
      }
    }
    if (orders.selectedTile !== null) {
      hexAt(orders.selectedTile).stroke({ width: Math.max(layout.size * 0.1, 2), color: 0xffffff });
    }
    if (orders.selectedTile !== null && orders.previewPath && orders.previewPath.length > 0) {
      const { x, y } = centerOf(orders.selectedTile);
      context.moveTo(x, y);
      for (const step of orders.previewPath) {
        const next = centerOf(step);
        context.lineTo(next.x, next.y);
      }
      context.stroke({ width: Math.max(layout.size * 0.08, 1.5), color: PATH_COLOR, alpha: PATH_ALPHA });
      const end = centerOf(orders.previewPath[orders.previewPath.length - 1]);
      context.circle(end.x, end.y, layout.size * 0.14).fill({ color: PATH_COLOR, alpha: PATH_ALPHA });
    }
    // settlements, buildings and units are not drawn here: `MapEntityRenderer` has their sprites,
    // one layer down
  };

  const drawOverlay = (pixi: PixiContext, world: WorldState, layout: HexLayout): DrawnOverlay => {
    const layer = new Container({ label: "world-overlay", zIndex: OVERLAY_Z_INDEX });
    // drawn twice side by side to wrap around east-west, like the map under it
    const copiesX = [0, wrapBounds(layout, world).period];

    const rows = Array.from({ length: world.rows }, () => new GraphicsContext());
    const entities = new GraphicsContext();
    for (const x of copiesX) {
      const copy = layer.addChild(new Container({ label: "fog-and-borders", x }));
      for (const context of rows) {
        copy.addChild(new Graphics({ context }));
      }
    }
    for (const x of copiesX) {
      layer.addChild(new Graphics({ context: entities, x }));
    }

    const drawn: DrawnOverlay = { layer, rows, entities };
    for (let row = 0; row < world.rows; row++) {
      paintRow(drawn, world, layout, row);
    }
    pixi.world.addChild(layer);
    return drawn;
  };
</script>

<script lang="ts">
  type WorldOverlayRendererProps = {
    pixi: PixiContext;
    world: WorldState;
    layout: HexLayout;
    /**
     * Row-major indices of the tiles a delta changed. `null` repaints the whole overlay, which is
     * also what a new `world` does.
     */
    dirtyTiles?: Uint32Array | null;
    /** Row-major index of the tile to outline, if any. */
    selectedTile?: number | null;
    /** Where the selected unit can get to, tinted in the viewer's colour. */
    reach?: ReachableData | null;
    /** The steps to the hovered tile, drawn as a line from the selected tile. */
    previewPath?: readonly number[] | null;
    /** The nation being played, for the reach's colour. */
    viewer?: number | null;
  };
  const {
    pixi,
    world,
    layout,
    dirtyTiles = null,
    selectedTile = null,
    reach = null,
    previewPath = null,
    viewer = null,
  }: WorldOverlayRendererProps = $props();

  let drawn = $state.raw<DrawnOverlay | null>(null);

  // the overlay is drawn whole when a full projection replaces the world
  $effect(() => {
    const currentDrawn = drawOverlay(pixi, world, layout);
    drawn = currentDrawn;
    return () => {
      drawn = null;
      currentDrawn.layer.destroy({ children: true });
      for (const context of currentDrawn.rows) {
        context.destroy();
      }
      currentDrawn.entities.destroy();
    };
  });

  // and row by row when a delta says which tiles moved
  $effect(() => {
    const changed = dirtyTiles;
    const orders: Orders = { selectedTile, reach, previewPath, viewer };
    const currentDrawn = drawn;
    if (!currentDrawn) {
      return;
    }
    if (changed) {
      // the indices come up in ascending order, so a row is painted once; out of order, the worst
      // that happens is painting one twice
      let paintedRow = -1;
      for (const index of changed) {
        if (index >= world.rows * world.columns) {
          continue;
        }
        const row = Math.floor(index / world.columns);
        if (row !== paintedRow) {
          paintRow(currentDrawn, world, layout, row);
          paintedRow = row;
        }
      }
    }
    // entities are sparse: redrawing them whole costs less than tracking which ones moved
    paintEntities(currentDrawn, world, layout, orders);
  });
</script>
