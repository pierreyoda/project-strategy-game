<script lang="ts" module>
  import { Container, Graphics, GraphicsContext } from "pixi.js";

  import type { PixiContext } from "../../../shared/pixi";
  import { type HexLayout, hexCenter, hexCorners, neighborAcross, wrapBounds } from "../../map/hexLayout";
  import { nationColor } from "../nations";
  import { NO_NATION, type ReachableData, TileVisibility } from "../session/protocol";
  import type { WorldState } from "../state/worldState";

  /** The unseen is drawn in the background's own color rather than in black, which would look like a hole. */
  const FOG_COLOR = 0x05070c;
  /** Opacity of the fog over a tile that was explored but is not watched right now. */
  const FOGGED_ALPHA = 0.55;
  /**
   * Who holds what, drawn in the political lens only: the normal map shows the ground. A tint over
   * the tiles a nation owns, a fainter one over its sphere of influence, and a line along each border
   * on the inside of each side's territory, in its own colour - as wide as `border` times a hex's radius.
   */
  const POLITICAL = { owner: 0.35, sphere: 0.15, border: 0.1, borderAlpha: 0.9 };
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
    /** Whether the rows show who holds what, as the political lens does. */
    political: boolean;
  };

  /**
   * The owner of a tile as far as borders go: `null` when the viewer has never explored it, since
   * nothing is known of whether a border runs there.
   */
  const knownOwner = (world: WorldState, col: number, row: number): number | null => {
    const index = row * world.columns + col;
    return world.visibility[index] === TileVisibility.Unexplored ? null : world.tileOwner[index];
  };

  const paintRow = (drawn: DrawnOverlay, world: WorldState, layout: HexLayout, row: number, political: boolean) => {
    const context = drawn.rows[row].clear();
    const corners = hexCorners(layout);
    const borderWidth = POLITICAL.border * layout.size;
    // the border's corners, pulled in for the line to lie along the inside of the edge
    const inner = corners.map((corner) => corner * (1 - borderWidth / (Math.sqrt(3) * layout.size)));
    for (let col = 0; col < world.columns; col++) {
      const index = row * world.columns + col;
      const visibility = world.visibility[index];
      const owner = world.tileOwner[index];
      const sphere = world.tileSphere[index];
      const { x, y } = hexCenter(layout, { col, row });
      const hex = () => context.poly(corners.map((corner, i) => corner + (i % 2 === 0 ? x : y)));
      const explored = visibility !== TileVisibility.Unexplored;

      if (political && explored && owner !== NO_NATION) {
        hex().fill({ color: nationColor(owner), alpha: POLITICAL.owner });
      }
      if (!explored) {
        hex().fill(FOG_COLOR);
      } else if (visibility === TileVisibility.Fogged) {
        hex().fill({ color: FOG_COLOR, alpha: FOGGED_ALPHA });
      }
      if (!political) {
        continue;
      }
      // over the fog: the viewer's own sphere reaches into what they have not explored yet, which
      // is the point of showing it
      if (sphere !== NO_NATION) {
        hex().fill({ color: nationColor(sphere), alpha: POLITICAL.sphere });
      }
      // and over the fog too, crisp: each side of a border draws its own line
      if (!explored || owner === NO_NATION) {
        continue;
      }
      for (let edge = 0; edge < 6; edge++) {
        const neighbor = neighborAcross(world, { col, row }, edge);
        const across = neighbor && knownOwner(world, neighbor.col, neighbor.row);
        if (across === null || across === owner) {
          continue;
        }
        const next = (edge + 1) % 6;
        context
          .moveTo(x + inner[2 * edge], y + inner[2 * edge + 1])
          .lineTo(x + inner[2 * next], y + inner[2 * next + 1])
          .stroke({ width: borderWidth, color: nationColor(owner), alpha: POLITICAL.borderAlpha, cap: "round" });
      }
    }
  };

  /** What the player is doing with a unit: the selection, where it can go, where the pointer would send it. */
  type Orders = {
    selectedTile: number | null;
    /** The tile of the focus view, outlined in the accent: distinct from the selection's white. */
    focusedTile: number | null;
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
    if (orders.focusedTile !== null) {
      // a soft glow under a crisp line, like the construction overlay's border
      const accent = parseInt(
        getComputedStyle(document.documentElement).getPropertyValue("--color-accent").trim().replace("#", ""),
        16,
      );
      hexAt(orders.focusedTile).stroke({ width: Math.max(layout.size * 0.3, 4), color: accent, alpha: 0.25 });
      hexAt(orders.focusedTile).stroke({ width: Math.max(layout.size * 0.08, 2), color: accent });
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

  const drawOverlay = (pixi: PixiContext, world: WorldState, layout: HexLayout, political: boolean): DrawnOverlay => {
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

    const drawn: DrawnOverlay = { layer, rows, entities, political };
    for (let row = 0; row < world.rows; row++) {
      paintRow(drawn, world, layout, row, political);
    }
    pixi.world.addChild(layer);
    return drawn;
  };
</script>

<script lang="ts">
  import { untrack } from "svelte";

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
    /** Row-major index of the focus view's tile, outlined in the accent. */
    focusedTile?: number | null;
    /** Where the selected unit can get to, tinted in the viewer's colour. */
    reach?: ReachableData | null;
    /** The steps to the hovered tile, drawn as a line from the selected tile. */
    previewPath?: readonly number[] | null;
    /** The nation being played, for the reach's colour. */
    viewer?: number | null;
    /** Draws who holds what - territories, spheres, borders - as the political lens does. */
    political?: boolean;
  };
  const {
    pixi,
    world,
    layout,
    dirtyTiles = null,
    selectedTile = null,
    focusedTile = null,
    reach = null,
    previewPath = null,
    viewer = null,
    political = false,
  }: WorldOverlayRendererProps = $props();

  let drawn = $state.raw<DrawnOverlay | null>(null);

  // the overlay is drawn whole when a full projection replaces the world
  $effect(() => {
    const currentDrawn = drawOverlay(
      pixi,
      world,
      layout,
      untrack(() => political),
    );
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

  // repainted whole when the political lens comes or goes
  $effect(() => {
    const showPolitical = political;
    const currentDrawn = drawn;
    if (!currentDrawn || currentDrawn.political === showPolitical) {
      return;
    }
    currentDrawn.political = showPolitical;
    for (let row = 0; row < world.rows; row++) {
      paintRow(currentDrawn, world, layout, row, showPolitical);
    }
  });

  // and row by row when a delta says which tiles moved
  $effect(() => {
    const changed = dirtyTiles;
    const orders: Orders = { selectedTile, focusedTile, reach, previewPath, viewer };
    const currentDrawn = drawn;
    if (!currentDrawn) {
      return;
    }
    if (changed) {
      // a border is drawn by the tiles on both of its sides, so a tile changing hands repaints the
      // rows above and below its own as well, where the political lens draws borders
      const spread = currentDrawn.political ? 1 : 0;
      // each row painted once, however many of its tiles changed
      const dirtyRows = new Uint8Array(world.rows);
      for (const index of changed) {
        if (index >= world.rows * world.columns) {
          continue;
        }
        const row = Math.floor(index / world.columns);
        dirtyRows.fill(1, Math.max(row - spread, 0), Math.min(row + spread + 1, world.rows));
      }
      for (let row = 0; row < world.rows; row++) {
        if (dirtyRows[row]) {
          paintRow(currentDrawn, world, layout, row, currentDrawn.political);
        }
      }
    }
    // entities are sparse: redrawing them whole costs less than tracking which ones moved
    paintEntities(currentDrawn, world, layout, orders);
  });
</script>
