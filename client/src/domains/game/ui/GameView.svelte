<script lang="ts">
  import { Viewport } from "pixi-viewport";
  import { Application } from "pixi.js";
  import { onMount } from "svelte";

  import HexMapRenderer from "../../map/components/HexMapRenderer.svelte";
  import { createHexLayout, mapHeight, wrapBounds } from "../../map/hexLayout";
  import type { HexMapData, OffsetCoords } from "../../map/hexMap";
  import type { PixiContext } from "../../../shared/pixi";

  type GameViewProps = {
    map: HexMapData;
    /** Draws the flat colors of the terrains only, without the textures of their tiles. */
    flatColors?: boolean;
    /** Called when the pointer moves to another tile, with `null` once it leaves the map. */
    onTileHover?: (tile: OffsetCoords | null) => void;
  };
  const { map, flatColors = false, onTileHover }: GameViewProps = $props();

  /** A hex is 64 world units tall. */
  const layout = createHexLayout(32);
  /** Hexes are displayed at most 256 pixels tall. */
  const MAX_ZOOM = 4;

  let host: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let camera = $state.raw<Viewport | null>(null);
  let pixi = $state.raw<PixiContext | null>(null);
  let screenWidth = $state(0);

  onMount(() => {
    let isMounted = true;
    const app = new Application();
    const initialized = app
      .init({
        canvas,
        resizeTo: host,
        antialias: true,
        autoDensity: true,
        resolution: window.devicePixelRatio,
        background: getComputedStyle(host).getPropertyValue("--color-ground").trim(),
      })
      .then(() => {
        if (!isMounted) {
          return;
        }
        const viewport = new Viewport({
          screenWidth: app.screen.width,
          screenHeight: app.screen.height,
          events: app.renderer.events,
        });
        viewport.drag().pinch().wheel().decelerate();
        app.stage.addChild(viewport);
        screenWidth = app.screen.width;
        app.renderer.on("resize", (width, height) => {
          viewport.resize(width, height);
          screenWidth = width;
        });
        camera = viewport;
        pixi = { app, world: viewport };
      });

    return () => {
      isMounted = false;
      camera = null;
      pixi = null;
      // an application cannot be destroyed before its initialization completes
      void initialized.then(() => app.destroy(false, { children: true }));
    };
  });

  // the camera starts zoomed out at the center of the map, and wraps around it east-west
  $effect(() => {
    if (!camera) {
      return;
    }
    const viewport = camera;
    const { period, minLeft, maxWidth } = wrapBounds(layout, map);
    const height = mapHeight(layout, map);
    viewport.worldWidth = 2 * period;
    viewport.worldHeight = height;
    viewport
      .clamp({ direction: "y", underflow: "center" })
      .setZoom(Math.max(viewport.screenWidth / maxWidth, viewport.screenHeight / height))
      .moveCenter(minLeft + period / 2, height / 2);

    const wrap = () => {
      if (viewport.left < minLeft) {
        viewport.left += period;
      } else if (viewport.left >= minLeft + period) {
        viewport.left -= period;
      }
    };
    viewport.on("moved", wrap).on("zoomed", wrap);
    return () => {
      viewport.off("moved", wrap).off("zoomed", wrap);
    };
  });

  // zoomed out, the camera shows the whole height of the map at most
  $effect(() => {
    if (!camera) {
      return;
    }
    // pixi-viewport ignores the scale limits along with the size ones: the zoom limit is a minimum width
    camera.clampZoom({
      minWidth: screenWidth / MAX_ZOOM,
      maxWidth: wrapBounds(layout, map).maxWidth,
      maxHeight: mapHeight(layout, map),
    });
  });
</script>

<div bind:this={host} class="size-full overflow-hidden">
  <canvas bind:this={canvas} class="block"></canvas>
  {#if pixi}
    <HexMapRenderer {pixi} {map} {layout} {flatColors} {onTileHover} />
  {/if}
</div>
