<script lang="ts" module>
  import { Container, Graphics, GraphicsContext, Sprite } from "pixi.js";
  import { onMount } from "svelte";

  import { type PixiContext, tokenColor } from "../../../shared/pixi";
  import {
    DEPOSIT_RESOURCES,
    type DepositLevel,
    type DepositResource,
    type DepositTextures,
    type MapDeposit,
    chipFrame,
    loadDepositTextures,
  } from "../deposits";
  import { entityCoords } from "../entities";
  import { type HexLayout, hexCenter, hexCorners, wrapBounds } from "../hexLayout";
  import type { HexMapData } from "../hexMap";

  /** Over everything else on the map, the fog included: the deposits are what the lens is for. See `GameView`. */
  const LENS_Z_INDEX = 11;
  /** The tint of a deposit's hex, stronger the richer it is, as on the "Resources" artboard. */
  const TINT_ALPHA: Record<DepositLevel, number> = { 1: 0.1, 2: 0.16, 3: 0.24 };
  /** The rim just inside the hex, in the resource's colour: `inset` scales the hex, `width` is a share of its radius. */
  const RIM = { inset: 0.944, width: 0.05, alpha: 0.85 };
  /** The chip's frame in the sprite sheet, the chip in its centre (see `art/deposits/deposits.mjs`). */
  const CHIP_FRAME = { width: 256, height: 112 };
  /**
   * How wide a chip's frame is drawn: at most this many pixels on screen, so that zooming in does
   * not blow it up, and at most this share of a hex's width, so that zooming out does not pile the
   * chips of neighbouring deposits onto each other - the tints carry the lens from there.
   */
  const CHIP_MAX_WIDTH = 64;
  const CHIP_HEX_SHARE = 0.9;

  type DrawnLens = {
    layer: Container;
    tints: GraphicsContext;
    chips: Sprite[];
  };

  const drawLens = (
    pixi: PixiContext,
    map: HexMapData,
    layout: HexLayout,
    textures: DepositTextures,
    deposits: readonly MapDeposit[],
  ): DrawnLens => {
    const layer = new Container({ label: "deposit-lens", zIndex: LENS_Z_INDEX });
    const colors = Object.fromEntries(
      DEPOSIT_RESOURCES.map((resource) => [resource, tokenColor(`--color-resource-${resource}`)]),
    ) as Record<DepositResource, number>;

    const corners = hexCorners(layout);
    const rim = corners.map((corner) => corner * RIM.inset);
    const tints = new GraphicsContext();
    for (const deposit of deposits) {
      const { x, y } = hexCenter(layout, entityCoords(map.columns, deposit.tile));
      const color = colors[deposit.resource];
      tints
        .poly(corners.map((corner, i) => corner + (i % 2 === 0 ? x : y)))
        .fill({ color, alpha: TINT_ALPHA[deposit.level] })
        .poly(rim.map((corner, i) => corner + (i % 2 === 0 ? x : y)))
        .stroke({ width: RIM.width * layout.size, color, alpha: RIM.alpha, join: "round" });
    }

    const chips: Sprite[] = [];
    // drawn twice side by side to wrap around east-west, like the map under it
    for (const x of [0, wrapBounds(layout, map).period]) {
      const copy = layer.addChild(new Container({ x }));
      copy.addChild(new Graphics({ context: tints }));
      for (const deposit of deposits) {
        const texture = textures[chipFrame(deposit)];
        if (!texture) {
          continue;
        }
        chips.push(
          copy.addChild(
            new Sprite({ texture, anchor: 0.5, position: hexCenter(layout, entityCoords(map.columns, deposit.tile)) }),
          ),
        );
      }
    }

    pixi.world.addChild(layer);
    return { layer, tints, chips };
  };
</script>

<script lang="ts">
  type DepositLensRendererProps = {
    pixi: PixiContext;
    map: HexMapData;
    layout: HexLayout;
    /** The deposits the lens shows, already filtered, one per tile. A new list redraws the layer. */
    deposits: readonly MapDeposit[];
  };
  const { pixi, map, layout, deposits }: DepositLensRendererProps = $props();

  let textures = $state.raw<DepositTextures | null>(null);
  onMount(() => {
    loadDepositTextures().then(
      (loadedTextures) => (textures = loadedTextures),
      (error) => console.error("Failed to load the deposit textures:", error),
    );
  });

  $effect(() => {
    if (!textures) {
      return;
    }
    // few deposits, and none of them moves: the layer is rebuilt whole when the list changes
    const drawn = drawLens(pixi, map, layout, textures, deposits);

    // the chips are sized for the screen rather than the world, so again whenever the zoom changes
    let sizedAt = Number.NaN;
    const sizeChips = () => {
      const scale = pixi.world.scale.x;
      if (scale === sizedAt) {
        return;
      }
      sizedAt = scale;
      const width = Math.min(CHIP_MAX_WIDTH / scale, CHIP_HEX_SHARE * layout.width);
      for (const chip of drawn.chips) {
        chip.setSize(width, (width * CHIP_FRAME.height) / CHIP_FRAME.width);
      }
    };
    sizeChips();
    pixi.app.ticker.add(sizeChips);

    return () => {
      pixi.app.ticker.remove(sizeChips);
      // the textures are shared, cached by PixiJS's `Assets`; the tints are this layer's own
      drawn.layer.destroy({ children: true });
      drawn.tints.destroy();
    };
  });
</script>
