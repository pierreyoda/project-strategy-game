<script lang="ts">
  import { Application, Container } from "pixi.js";
  import { type Snippet, onMount } from "svelte";

  import type { PixiContext } from "../../../shared/pixi";
  import { type HexLayout, createHexLayout, mapHeight } from "../hexLayout";
  import type { HexMapData } from "../hexMap";
  import HexMapRenderer from "./HexMapRenderer.svelte";

  type MapStoryStageProps = {
    /** A small patch of map: the stage is exactly its size, at `zoom`. */
    map: HexMapData;
    /** Fixed for the stage's life, like a camera that does not move: 1 draws hexes 64 px tall, as `GameView` does. */
    zoom?: number;
    /** What the story draws over the terrain, in the same world. */
    children?: Snippet<[PixiContext, HexLayout]>;
  };
  const { map, zoom = 1, children }: MapStoryStageProps = $props();

  /** `GameView`'s own layout, so that the renderers are drawn at the sizes the game draws them. */
  const layout = createHexLayout(32);
  // the odd rows stick out half a hex to the east
  const width = $derived(Math.ceil(layout.width * (map.columns + 0.5) * zoom));
  const height = $derived(Math.ceil(mapHeight(layout, map) * zoom));

  let canvas: HTMLCanvasElement;
  let pixi = $state.raw<PixiContext | null>(null);

  onMount(() => {
    let isMounted = true;
    const app = new Application();
    const initialized = app
      .init({
        canvas,
        width,
        height,
        // like `GameView`, whose map renderer's ground shader is GLSL only
        preference: "webgl",
        antialias: true,
        autoDensity: true,
        resolution: window.devicePixelRatio,
        background: getComputedStyle(document.documentElement).getPropertyValue("--color-ground").trim(),
      })
      .then(() => {
        if (!isMounted) {
          return;
        }
        // the layers draw in `zIndex` order, as on `GameView`'s viewport
        const world = app.stage.addChild(new Container({ label: "world", scale: zoom, sortableChildren: true }));
        pixi = { app, world };
      });

    return () => {
      isMounted = false;
      pixi = null;
      void initialized.then(() => app.destroy(false, { children: true }));
    };
  });
</script>

<!--
  A map renderer's harness for Storybook: a patch of real, textured map under whatever the story draws,
  at a fixed zoom. Not for the game: it neither pans, zooms nor wraps around.
-->
<canvas bind:this={canvas} class="block" style:width="{width}px" style:height="{height}px"></canvas>
{#if pixi}
  <HexMapRenderer {pixi} {map} {layout} />
  {@render children?.(pixi, layout)}
{/if}
