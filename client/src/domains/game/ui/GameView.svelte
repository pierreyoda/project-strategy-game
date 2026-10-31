<script lang="ts">
  import { Viewport } from "pixi-viewport";
  import { Application } from "pixi.js";
  import { onMount } from "svelte";

  import HexMapRenderer from "../../map/components/HexMapRenderer.svelte";
  import MapEntityRenderer from "../../map/components/MapEntityRenderer.svelte";
  import NationsOverlayRenderer from "../../map/components/NationsOverlayRenderer.svelte";
  import type { MapEntity } from "../../map/entities";
  import { createHexLayout, hexCenter, mapHeight, wrapBounds } from "../../map/hexLayout";
  import type { HexMapData, OffsetCoords } from "../../map/hexMap";
  import type { MapLook } from "../../map/mapLook";
  import type { PlanetNations } from "../../map/procgen/planetGenerator";
  import type { PixiContext } from "../../../shared/pixi";
  import type { ReachableData } from "../session/protocol";
  import type { WorldState } from "../state/worldState";
  import WorldOverlayRenderer from "./WorldOverlayRenderer.svelte";

  type GameViewProps = {
    map: HexMapData;
    /**
     * The session's world, drawn over the map: fog, borders, units and settlements. Left out when
     * there is no session to show, as in the planet generator's test bench.
     */
    world?: WorldState | null;
    /**
     * Generated nations drawn over the map as they came out of the generator - spheres, turn-1
     * borders, capitals - for the planet generator's test bench, which has no session.
     */
    nations?: PlanetNations | null;
    /**
     * Row-major indices of the tiles a projection's delta changed, repainted on their own. A new
     * `map` or `world` repaints everything instead.
     */
    dirtyTiles?: Uint32Array | null;
    /**
     * Settlements and the like, drawn on the tiles with their colors already resolved. The world's
     * own are not derived here: which stage a settlement is at is the simulation's to say.
     */
    entities?: readonly MapEntity[];
    /** Draws the flat colors of the terrains only, without the textures of their tiles. */
    flatColors?: boolean;
    /** The knobs of the textured tiles' variety, for the test bench to turn; the game keeps the default. */
    look?: MapLook;
    /** Called when the pointer moves to another tile, with `null` once it leaves the map. */
    onTileHover?: (tile: OffsetCoords | null) => void;
    /** Called when a tile is clicked. */
    onTileClick?: (tile: OffsetCoords) => void;
    /** Row-major index of the tile whose unit is selected, outlined over the map. */
    selectedTile?: number | null;
    /** Where the selected unit can get to, as the simulation answered; tinted over the map. */
    reach?: ReachableData | null;
    /** The steps to the hovered tile, previewed from the selected tile. */
    previewPath?: readonly number[] | null;
    /** The nation being played, whose colour the reach takes. */
    viewer?: number | null;
    /**
     * Row-major index of a tile to frame the camera on, whenever it changes. A new game starts with
     * almost nothing explored, so the whole map is not a useful first view.
     */
    focusTile?: number | null;
  };
  const {
    map,
    world = null,
    nations = null,
    dirtyTiles = null,
    entities = [],
    flatColors = false,
    look,
    onTileHover,
    onTileClick,
    selectedTile = null,
    reach = null,
    previewPath = null,
    viewer = null,
    focusTile = null,
  }: GameViewProps = $props();

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
        // the layers draw in `zIndex` order rather than in the order they were added: a renderer
        // that rebuilds its layer - as the map does once its textures load - would otherwise land
        // back on top of everything drawn over it
        viewport.sortableChildren = true;
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

  // declared after the effect framing the whole map, so that it has the last word on a new game
  $effect(() => {
    const tile = focusTile;
    if (!camera || tile === null) {
      return;
    }
    const { x, y } = hexCenter(layout, { col: tile % map.columns, row: Math.floor(tile / map.columns) });
    // hexes at their texture's own scale, close enough to tell what is on them
    camera.setZoom(1).moveCenter(x, y);
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
    <HexMapRenderer {pixi} {map} {layout} {dirtyTiles} {flatColors} {look} {onTileHover} {onTileClick} />
    <MapEntityRenderer {pixi} {map} {layout} {entities} />
    {#if world}
      <WorldOverlayRenderer {pixi} {world} {layout} {dirtyTiles} {selectedTile} {reach} {previewPath} {viewer} />
    {/if}
    {#if nations}
      <NationsOverlayRenderer {pixi} {map} {nations} {layout} />
    {/if}
  {/if}
</div>
