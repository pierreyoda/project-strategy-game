<script lang="ts" module>
  import { Container, Graphics, GraphicsContext } from "pixi.js";

  import type { PixiContext } from "../../../shared/pixi";
  import { nationColor } from "../../game/nations";
  import { NO_NATION } from "../../game/session/protocol";
  import { type HexLayout, hexCenter, hexCorners, wrapBounds } from "../hexLayout";
  import type { HexMapData } from "../hexMap";
  import type { PlanetNations } from "../procgen/planetGenerator";

  /** Opacity of the tint over a nation's sphere of influence. */
  const SPHERE_TINT_ALPHA = 0.14;
  /** Opacity of the tint over the land a nation owns on turn 1, like the session view's borders. */
  const OWNER_TINT_ALPHA = 0.4;
  /** Drawn over the map, like the session's overlay. */
  const OVERLAY_Z_INDEX = 10;

  /**
   * Draw the nations whole: the generator's output never changes once it is on screen, so there is
   * nothing to repaint row by row.
   */
  const drawNations = (pixi: PixiContext, map: HexMapData, nations: PlanetNations, layout: HexLayout) => {
    const context = new GraphicsContext();
    const corners = hexCorners(layout);
    const hexAt = (index: number) => {
      const { x, y } = hexCenter(layout, { col: index % map.columns, row: Math.floor(index / map.columns) });
      return { x, y, poly: () => context.poly(corners.map((corner, i) => corner + (i % 2 === 0 ? x : y))) };
    };

    for (let index = 0; index < nations.tileSphere.length; index++) {
      const sphere = nations.tileSphere[index];
      if (sphere === NO_NATION) {
        continue;
      }
      const owner = nations.tileOwner[index];
      hexAt(index)
        .poly()
        .fill({ color: nationColor(sphere), alpha: owner === NO_NATION ? SPHERE_TINT_ALPHA : OWNER_TINT_ALPHA });
    }
    for (const [nation, tile] of nations.capitals.entries()) {
      const { x, y } = hexAt(tile);
      context
        .circle(x, y, layout.size * 0.45)
        .stroke({ width: Math.max(layout.size * 0.12, 2), color: nationColor(nation) })
        .circle(x, y, layout.size * 0.18)
        .fill(nationColor(nation));
    }

    // drawn twice side by side to wrap around east-west, like the map under it
    const layer = new Container({ label: "nations-overlay", zIndex: OVERLAY_Z_INDEX });
    for (const x of [0, wrapBounds(layout, map).period]) {
      layer.addChild(new Graphics({ context, x }));
    }
    pixi.world.addChild(layer);
    return { layer, context };
  };
</script>

<script lang="ts">
  type NationsOverlayRendererProps = {
    pixi: PixiContext;
    map: HexMapData;
    nations: PlanetNations;
    layout: HexLayout;
  };
  const { pixi, map, nations, layout }: NationsOverlayRendererProps = $props();

  $effect(() => {
    const drawn = drawNations(pixi, map, nations, layout);
    return () => {
      drawn.layer.destroy({ children: true });
      drawn.context.destroy();
    };
  });
</script>
