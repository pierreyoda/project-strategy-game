<script lang="ts" module>
  import { Container, Graphics, GraphicsContext, Text } from "pixi.js";
  import { onMount } from "svelte";

  import { type PixiContext, tokenColor } from "../../../shared/pixi";
  import { type MapConstruction, constructionProgress } from "../constructions";
  import { entityCoords } from "../entities";
  import { type HexLayout, hexCenter, hexCorners, wrapBounds } from "../hexLayout";
  import type { HexMapData } from "../hexMap";
  import { DIMMED_ALPHA } from "../lensLook";

  /** Over the entities, whose building the plate dims, and under the fog. See `GameView`. */
  const CONSTRUCTIONS_Z_INDEX = 6;
  /** The plate's radius as a share of the hex's, leaving the tile's edge to its neighbors. */
  const PLATE_SCALE = 0.9;
  /** Width of the progress border, as a share of the hex's radius. */
  const BORDER_WIDTH = 0.07;
  /**
   * Hex heights on screen, in pixels, over which the number and its label fade in: under them the text
   * is too small to read, and the plate and its border carry the meaning alone.
   */
  const NUMBER_FADE_IN = { from: 16, to: 24 };
  /** Fully shown by `GameView`'s ×1, where a new game opens: hexes 64 px tall, the label about 9 px. */
  const LABEL_FADE_IN = { from: 56, to: 64 };
  /** Opacity of the plate: the building under it stays in view, the number still reads over it. */
  const PLATE_ALPHA = 0.4;
  /** Size of the number, as a share of the hex's radius; two digits shrink to clear the plate's sides. */
  const NUMBER_SIZE = { digit: 0.95, digits: 0.95 * 0.75 };
  /**
   * Resolution the text is rasterized at: sharp up to `GameView`'s 4× zoom on a 2× screen. A handful of
   * constructions at a time, so the textures stay small.
   */
  const TEXT_RESOLUTION = 8;

  /** Corners of the plate centered at (0, 0), clockwise from the top one, as points. */
  const plateCorners = (layout: HexLayout): { x: number; y: number }[] => {
    const flat = hexCorners(layout);
    return Array.from({ length: 6 }, (_, i) => ({ x: flat[2 * i] * PLATE_SCALE, y: flat[2 * i + 1] * PLATE_SCALE }));
  };

  /**
   * Trace the share `progress` of the plate's outline, clockwise from its top corner: the hexagon's six
   * sides are equal, so the share of the perimeter is the share of the sides.
   */
  const traceProgress = (context: GraphicsContext, corners: readonly { x: number; y: number }[], progress: number) => {
    const sides = progress * 6;
    context.moveTo(corners[0].x, corners[0].y);
    for (let side = 0; side < Math.ceil(sides); side++) {
      const [from, to] = [corners[side], corners[(side + 1) % 6]];
      const t = Math.min(sides - side, 1);
      context.lineTo(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
    }
  };

  type DrawnConstructions = {
    layer: Container;
    numbers: Text[];
    labels: Text[];
    /** The plate and its track, the same for every construction. */
    plate: GraphicsContext;
    /** Each construction's progress, its own. */
    borders: GraphicsContext[];
  };

  const drawConstructions = (
    pixi: PixiContext,
    map: HexMapData,
    layout: HexLayout,
    constructions: readonly MapConstruction[],
    fontsReady: boolean,
  ): DrawnConstructions => {
    const layer = new Container({ label: "constructions", zIndex: CONSTRUCTIONS_Z_INDEX });
    const colors = {
      ground: tokenColor("--color-ground"),
      fg: tokenColor("--color-fg"),
      subtle: tokenColor("--color-fg-subtle"),
      accent: tokenColor("--color-accent"),
    };
    const corners = plateCorners(layout);
    const width = BORDER_WIDTH * layout.size;

    // a dimmed hexagon over the tile and the faint track the progress runs along
    const plate = new GraphicsContext()
      .poly(corners)
      .fill({ color: colors.ground, alpha: PLATE_ALPHA })
      .poly(corners)
      .stroke({ width, color: colors.fg, alpha: 0.12, join: "round" });

    const drawn: DrawnConstructions = { layer, numbers: [], labels: [], plate, borders: [] };
    // drawn twice side by side to wrap around east-west, like the map under it
    for (const x of [0, wrapBounds(layout, map).period]) {
      const copy = layer.addChild(new Container({ x }));
      for (const construction of constructions) {
        const site = copy.addChild(
          new Container({ position: hexCenter(layout, entityCoords(map.columns, construction.tile)) }),
        );
        site.addChild(new Graphics({ context: plate }));

        // the share done, in the accent, clockwise from the top: a soft glow under a crisp line
        const progress = constructionProgress(construction);
        if (progress > 0) {
          const border = new GraphicsContext();
          traceProgress(border, corners, progress);
          border.stroke({ width: 3 * width, color: colors.accent, alpha: 0.25, cap: "round", join: "round" });
          traceProgress(border, corners, progress);
          border.stroke({ width, color: colors.accent, cap: "round", join: "round" });
          drawn.borders.push(border);
          site.addChild(new Graphics({ context: border }));
        }

        if (!fontsReady) {
          continue;
        }
        // the last turn in the accent: it completes at the end of this one
        const last = construction.turnsLeft <= 1;
        const number = site.addChild(
          new Text({
            text: String(construction.turnsLeft),
            anchor: 0.5,
            y: -0.14 * layout.size,
            resolution: TEXT_RESOLUTION,
            style: {
              fontFamily: "Rajdhani",
              fontWeight: "700",
              fontSize: (construction.turnsLeft >= 10 ? NUMBER_SIZE.digits : NUMBER_SIZE.digit) * layout.size,
              fill: last ? colors.accent : colors.fg,
            },
          }),
        );
        const label = site.addChild(
          new Text({
            text: last ? "TURN" : "TURNS",
            anchor: 0.5,
            y: 0.4 * layout.size,
            resolution: TEXT_RESOLUTION,
            style: {
              fontFamily: "Rajdhani",
              fontWeight: "600",
              fontSize: 0.28 * layout.size,
              letterSpacing: 0.03 * layout.size,
              fill: colors.subtle,
            },
          }),
        );
        drawn.numbers.push(number);
        drawn.labels.push(label);
      }
    }

    pixi.world.addChild(layer);
    return drawn;
  };

  const fade = (value: number, { from, to }: { from: number; to: number }) =>
    Math.min(Math.max((value - from) / (to - from), 0), 1);
</script>

<script lang="ts">
  type TileConstructionTurnsOverlayProps = {
    pixi: PixiContext;
    map: HexMapData;
    layout: HexLayout;
    /** The buildings going up, and how long each has left. A new list redraws the layer. */
    constructions: readonly MapConstruction[];
    /** Steps the overlay back, for a lens to bring something else forward. */
    dimmed?: boolean;
  };
  const { pixi, map, layout, constructions, dimmed = false }: TileConstructionTurnsOverlayProps = $props();

  let layer = $state.raw<Container | null>(null);
  $effect(() => {
    if (layer) {
      layer.alpha = dimmed ? DIMMED_ALPHA : 1;
    }
  });

  // PixiJS draws text on a canvas, which uses a font only once the page has it: the stylesheet's
  // Rajdhani is loaded on first use, which a canvas does not count as
  let fontsReady = $state(false);
  onMount(() => {
    Promise.all([document.fonts.load("700 16px Rajdhani"), document.fonts.load("600 16px Rajdhani")]).then(
      () => (fontsReady = true),
      (error) => console.error("Failed to load the construction overlay's font:", error),
    );
  });

  $effect(() => {
    const drawn = drawConstructions(pixi, map, layout, constructions, fontsReady);
    layer = drawn.layer;

    // the number is readable well before its label: each fades in once large enough on screen
    const updateDetail = () => {
      const hexHeight = 2 * layout.size * pixi.world.scale.y;
      const [numberAlpha, labelAlpha] = [fade(hexHeight, NUMBER_FADE_IN), fade(hexHeight, LABEL_FADE_IN)];
      for (const number of drawn.numbers) {
        number.alpha = numberAlpha;
        number.visible = numberAlpha > 0;
      }
      for (const label of drawn.labels) {
        label.alpha = labelAlpha;
        label.visible = labelAlpha > 0;
      }
    };
    pixi.app.ticker.add(updateDetail);

    return () => {
      pixi.app.ticker.remove(updateDetail);
      layer = null;
      drawn.layer.destroy({ children: true });
      drawn.plate.destroy();
      for (const border of drawn.borders) {
        border.destroy();
      }
    };
  });
</script>
