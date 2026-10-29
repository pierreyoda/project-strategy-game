<script lang="ts" module>
  import { Container, type FederatedPointerEvent, Graphics, GraphicsContext, Sprite } from "pixi.js";
  import { onMount } from "svelte";

  import type { PixiContext } from "../../../shared/pixi";
  import { type HexLayout, hexAt, hexCenter, hexCorners, wrapBounds, wrapHex } from "../hexLayout";
  import type { HexMapData, OffsetCoords } from "../hexMap";
  import { TERRAIN_COLORS, type TerrainKind } from "../terrain";
  import { type TerrainTextures, loadTerrainTextures } from "../terrainTextures";
  import { pickTileVariant } from "../tileset";

  /** Heights of a hex on screen, in pixels, over which the textured tiles fade in over the flat colors. */
  const TEXTURES_FADE_IN = { from: 16, to: 32 };
  /**
   * Scale of the textured tiles, slightly enlarged to overlap their neighbors: the antialiased edges of the tiles
   * would otherwise let the flat colors show between them.
   */
  const TEXTURED_TILE_SCALE = 1.02;

  const drawFlatTiles = (map: HexMapData, layout: HexLayout): GraphicsContext => {
    const tiles = new GraphicsContext();
    const corners = hexCorners(layout);
    for (let row = 0; row < map.rows; row++) {
      for (let col = 0; col < map.columns; col++) {
        const { x, y } = hexCenter(layout, { col, row });
        const terrain = map.terrain[row * map.columns + col] as TerrainKind;
        tiles
          .poly(corners.map((corner, i) => corner + (i % 2 === 0 ? x : y)))
          .fill(TERRAIN_COLORS[terrain])
          .stroke({ width: 1, color: 0x000000, alpha: 0.15 });
      }
    }
    return tiles;
  };

  const createTexturedTiles = (map: HexMapData, layout: HexLayout, textures: TerrainTextures): Container => {
    const tiles = new Container();
    for (let row = 0; row < map.rows; row++) {
      for (let col = 0; col < map.columns; col++) {
        const terrain = map.terrain[row * map.columns + col] as TerrainKind;
        const texture = pickTileVariant(textures[terrain], { col, row });
        const tile = tiles.addChild(new Sprite({ texture, anchor: 0.5, position: hexCenter(layout, { col, row }) }));
        tile.setSize(layout.width * TEXTURED_TILE_SCALE, 2 * layout.size * TEXTURED_TILE_SCALE);
      }
    }
    return tiles;
  };
</script>

<script lang="ts">
  type HexMapRendererProps = {
    pixi: PixiContext;
    map: HexMapData;
    layout: HexLayout;
    /** Draws the flat colors of the terrains only, without the textures of their tiles. */
    flatColors?: boolean;
    /** Called when the pointer moves to another tile, with `null` once it leaves the map. */
    onTileHover?: (tile: OffsetCoords | null) => void;
  };
  const { pixi, map, layout, flatColors = false, onTileHover }: HexMapRendererProps = $props();

  let textures = $state.raw<TerrainTextures | null>(null);
  onMount(() => {
    loadTerrainTextures().then(
      (loadedTextures) => (textures = loadedTextures),
      (error) => console.error("Failed to load the terrain textures:", error),
    );
  });

  $effect(() => {
    const layer = new Container({ label: "hex-map" });
    // drawn twice side by side to wrap around east-west, see `wrapBounds`
    const copiesX = [0, wrapBounds(layout, map).period];

    const flatTiles = drawFlatTiles(map, layout);
    for (const x of copiesX) {
      layer.addChild(new Graphics({ context: flatTiles, x }));
    }
    // over the flat colors, which also fill any gap between the textured tiles
    const texturedTiles = layer.addChild(new Container({ label: "textured-tiles", isRenderGroup: true }));
    if (textures) {
      for (const x of copiesX) {
        texturedTiles.addChild(createTexturedTiles(map, layout, textures)).x = x;
      }
    }

    const hoverOutline = new GraphicsContext().poly(hexCorners(layout)).stroke({ width: 3, color: 0xffffff });
    const hoverMarker = layer.addChild(new Graphics({ context: hoverOutline, visible: false }));
    pixi.world.addChild(layer);

    // the textured tiles fade in once hexes are large enough on screen for their details
    const updateTexturesOpacity = () => {
      const hexHeight = 2 * layout.size * pixi.world.scale.y;
      const fadeIn = (hexHeight - TEXTURES_FADE_IN.from) / (TEXTURES_FADE_IN.to - TEXTURES_FADE_IN.from);
      texturedTiles.alpha = flatColors ? 0 : Math.min(Math.max(fadeIn, 0), 1);
      texturedTiles.visible = texturedTiles.alpha > 0;
    };
    pixi.app.ticker.add(updateTexturesOpacity);

    let hoveredTile: OffsetCoords | null = null;
    const hover = (tile: OffsetCoords | null) => {
      if (tile?.col !== hoveredTile?.col || tile?.row !== hoveredTile?.row) {
        hoveredTile = tile;
        onTileHover?.(tile);
      }
    };
    const onPointerMove = (event: FederatedPointerEvent) => {
      // unwrapped, to outline the hex actually under the pointer, in either copy of the map
      const hex = hexAt(layout, event.getLocalPosition(layer));
      const tile = wrapHex(map, hex);
      hoverMarker.visible = tile !== null;
      hoverMarker.position = hexCenter(layout, hex);
      hover(tile);
    };
    const onPointerLeave = () => {
      hoverMarker.visible = false;
      hover(null);
    };
    pixi.world.on("pointermove", onPointerMove).on("pointerleave", onPointerLeave);

    return () => {
      pixi.app.ticker.remove(updateTexturesOpacity);
      pixi.world.off("pointermove", onPointerMove).off("pointerleave", onPointerLeave);
      // the textures are shared, cached by PixiJS's `Assets`
      layer.destroy({ children: true });
      flatTiles.destroy();
      hoverOutline.destroy();
    };
  });
</script>
