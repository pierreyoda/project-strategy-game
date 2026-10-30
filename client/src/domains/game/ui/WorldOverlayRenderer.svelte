<script lang="ts" module>
  import { Container, Graphics, GraphicsContext } from "pixi.js";

  import type { PixiContext } from "../../../shared/pixi";
  import { type HexLayout, hexCenter, hexCorners, wrapBounds } from "../../map/hexLayout";
  import { nationColor } from "../nations";
  import { NO_NATION, TileVisibility } from "../session/protocol";
  import type { WorldState } from "../state/worldState";

  /** The unseen is drawn in the background's own color rather than in black, which would look like a hole. */
  const FOG_COLOR = 0x05070c;
  /** Opacity of the fog over a tile that was explored but is not watched right now. */
  const FOGGED_ALPHA = 0.55;
  /** Opacity of the tint marking a tile's owner. */
  const OWNER_TINT_ALPHA = 0.2;
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
    /** Units and settlements: few enough to redraw whole, rather than tracked one by one. */
    entities: GraphicsContext;
  };

  const paintRow = (drawn: DrawnOverlay, world: WorldState, layout: HexLayout, row: number) => {
    const context = drawn.rows[row].clear();
    const corners = hexCorners(layout);
    for (let col = 0; col < world.columns; col++) {
      const index = row * world.columns + col;
      const visibility = world.visibility[index];
      const owner = world.tileOwner[index];
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
    }
  };

  const paintEntities = (drawn: DrawnOverlay, world: WorldState, layout: HexLayout, selectedTile: number | null) => {
    const context = drawn.entities.clear();
    const outline = { width: Math.max(layout.size * 0.06, 1), color: FOG_COLOR };
    const centerOf = (tile: number) =>
      hexCenter(layout, { col: tile % world.columns, row: Math.floor(tile / world.columns) });

    if (selectedTile !== null) {
      const { x, y } = centerOf(selectedTile);
      context
        .poly(hexCorners(layout).map((corner, i) => corner + (i % 2 === 0 ? x : y)))
        .stroke({ width: Math.max(layout.size * 0.1, 2), color: 0xffffff });
    }

    for (const { tile, owner } of world.settlements.values()) {
      const { x, y } = centerOf(tile);
      const side = layout.size * 0.8;
      context
        .roundRect(x - side / 2, y - side / 2, side, side, side * 0.2)
        .fill(nationColor(owner))
        .stroke(outline);
    }
    for (const { tile, owner } of world.units.values()) {
      const { x, y } = centerOf(tile);
      context
        .circle(x, y, layout.size * 0.3)
        .fill(nationColor(owner))
        .stroke(outline);
    }
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
  };
  const { pixi, world, layout, dirtyTiles = null, selectedTile = null }: WorldOverlayRendererProps = $props();

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
    const selected = selectedTile;
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
    paintEntities(currentDrawn, world, layout, selected);
  });
</script>
