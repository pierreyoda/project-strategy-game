<script lang="ts" module>
  import { Container, Sprite } from "pixi.js";
  import { onMount } from "svelte";

  import type { PixiContext } from "../../../shared/pixi";
  import { type DepositTextures, type MapDeposit, depositFrame, loadDepositTextures } from "../deposits";
  import { entityCoords } from "../entities";
  import { type HexLayout, hexCenter, wrapBounds } from "../hexLayout";
  import type { HexMapData } from "../hexMap";

  /**
   * Heights of a hex on screen, in pixels, over which the deposits fade in: the window of the
   * tiles' own textures and of the entities' sprites, since a deposit is part of the ground they
   * draw. Below it the map is a diagram, and the resource lens is where deposits are read from.
   */
  const DEPOSITS_FADE_IN = { from: 16, to: 32 };
  /** On the ground: over the tiles, under everything standing on them and under the fog. See `GameView`. */
  const DEPOSITS_Z_INDEX = 2;

  const drawDeposits = (
    pixi: PixiContext,
    map: HexMapData,
    layout: HexLayout,
    textures: DepositTextures,
    deposits: readonly MapDeposit[],
  ): Container => {
    const layer = new Container({ label: "map-deposits", zIndex: DEPOSITS_Z_INDEX, isRenderGroup: true });
    // drawn twice side by side to wrap around east-west, like the map under it
    for (const x of [0, wrapBounds(layout, map).period]) {
      const copy = layer.addChild(new Container({ x }));
      for (const deposit of deposits) {
        const texture = textures[depositFrame(deposit)];
        if (!texture) {
          continue;
        }
        const sprite = new Sprite({
          texture,
          anchor: 0.5,
          position: hexCenter(layout, entityCoords(map.columns, deposit.tile)),
        });
        // after the texture, which sizes the sprite to the texture's own dimensions
        sprite.setSize(layout.width, 2 * layout.size);
        copy.addChild(sprite);
      }
    }
    pixi.world.addChild(layer);
    return layer;
  };
</script>

<script lang="ts">
  type DepositRendererProps = {
    pixi: PixiContext;
    map: HexMapData;
    layout: HexLayout;
    /** The deposits the viewer knows of, one per tile. A new list redraws the layer. */
    deposits: readonly MapDeposit[];
  };
  const { pixi, map, layout, deposits }: DepositRendererProps = $props();

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
    // deposits do not move and are few: the layer is rebuilt whole when the list changes
    const layer = drawDeposits(pixi, map, layout, textures, deposits);

    const updateDetail = () => {
      const hexHeight = 2 * layout.size * pixi.world.scale.y;
      const fadeIn = (hexHeight - DEPOSITS_FADE_IN.from) / (DEPOSITS_FADE_IN.to - DEPOSITS_FADE_IN.from);
      layer.alpha = Math.min(Math.max(fadeIn, 0), 1);
      layer.visible = layer.alpha > 0;
    };
    updateDetail();
    pixi.app.ticker.add(updateDetail);

    return () => {
      pixi.app.ticker.remove(updateDetail);
      // the textures are shared, cached by PixiJS's `Assets`
      layer.destroy({ children: true });
    };
  });
</script>
