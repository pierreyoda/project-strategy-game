<script lang="ts" module>
  import { Container, type FederatedPointerEvent, Graphics, GraphicsContext, Sprite, Texture } from "pixi.js";
  import { onMount } from "svelte";

  import type { PixiContext } from "../../../shared/pixi";
  import { type HexLayout, hexAt, hexCenter, hexCorners, wrapBounds, wrapHex } from "../hexLayout";
  import type { HexMapData, OffsetCoords } from "../hexMap";
  import { TERRAIN_COLORS, isExplored } from "../terrain";
  import { type TerrainTextures, loadTerrainTextures } from "../terrainTextures";
  import { pickTileVariant } from "../tileset";

  /** Heights of a hex on screen, in pixels, over which the textured tiles fade in over the flat colors. */
  const TEXTURES_FADE_IN = { from: 16, to: 32 };
  /**
   * Scale of the textured tiles, slightly enlarged to overlap their neighbors: the antialiased edges of the tiles
   * would otherwise let the flat colors show between them.
   */
  const TEXTURED_TILE_SCALE = 1.02;
  /** The map draws under everything the game puts over it, see `GameView`. */
  const MAP_Z_INDEX = 0;

  /**
   * The drawn map, kept addressable tile by tile.
   *
   * A session hands the renderer the tiles that changed rather than a new map, and the whole point of
   * carrying them is to repaint those alone: one shared `GraphicsContext` per row for the flat colors,
   * and one sprite per tile for the textures.
   */
  type DrawnMap = {
    layer: Container;
    /** Flat colors, one context per row, each drawn once per copy of the map. */
    rows: GraphicsContext[];
    texturedTiles: Container;
    /** Textured tiles, one sprite per tile indexed row-major, one array per copy of the map. */
    tileSprites: Sprite[][];
  };

  const paintRow = (drawn: DrawnMap, map: HexMapData, layout: HexLayout, row: number) => {
    const context = drawn.rows[row].clear();
    const corners = hexCorners(layout);
    for (let col = 0; col < map.columns; col++) {
      const terrain = map.terrain[row * map.columns + col];
      // nothing is known of an unexplored tile, so nothing is drawn of it
      if (!isExplored(terrain)) {
        continue;
      }
      const { x, y } = hexCenter(layout, { col, row });
      context
        .poly(corners.map((corner, i) => corner + (i % 2 === 0 ? x : y)))
        .fill(TERRAIN_COLORS[terrain])
        .stroke({ width: 1, color: 0x000000, alpha: 0.15 });
    }
  };

  const paintTexturedTile = (
    drawn: DrawnMap,
    map: HexMapData,
    layout: HexLayout,
    textures: TerrainTextures | null,
    index: number,
  ) => {
    const terrain = map.terrain[index];
    const coords = { col: index % map.columns, row: Math.floor(index / map.columns) };
    const texture = textures && isExplored(terrain) ? pickTileVariant(textures[terrain], coords) : null;
    for (const sprites of drawn.tileSprites) {
      const sprite = sprites[index];
      sprite.visible = texture !== null;
      if (texture) {
        sprite.texture = texture;
        // after the texture, which resets the sprite to the texture's own size
        sprite.setSize(layout.width * TEXTURED_TILE_SCALE, 2 * layout.size * TEXTURED_TILE_SCALE);
      }
    }
  };

  const drawMap = (
    pixi: PixiContext,
    map: HexMapData,
    layout: HexLayout,
    textures: TerrainTextures | null,
  ): DrawnMap => {
    const layer = new Container({ label: "hex-map", zIndex: MAP_Z_INDEX });
    // drawn twice side by side to wrap around east-west, see `wrapBounds`
    const copiesX = [0, wrapBounds(layout, map).period];

    const rows = Array.from({ length: map.rows }, () => new GraphicsContext());
    for (const x of copiesX) {
      const copy = layer.addChild(new Container({ label: "flat-tiles", x }));
      for (const context of rows) {
        copy.addChild(new Graphics({ context }));
      }
    }

    // over the flat colors, which also fill any gap between the textured tiles
    const texturedTiles = layer.addChild(new Container({ label: "textured-tiles", isRenderGroup: true }));
    const tileSprites = copiesX.map((x) => {
      const copy = texturedTiles.addChild(new Container({ x }));
      return Array.from({ length: map.rows * map.columns }, (_, index) =>
        copy.addChild(
          new Sprite({
            texture: Texture.EMPTY,
            anchor: 0.5,
            visible: false,
            position: hexCenter(layout, { col: index % map.columns, row: Math.floor(index / map.columns) }),
          }),
        ),
      );
    });

    const drawn: DrawnMap = { layer, rows, texturedTiles, tileSprites };
    for (let row = 0; row < map.rows; row++) {
      paintRow(drawn, map, layout, row);
    }
    for (let index = 0; index < map.rows * map.columns; index++) {
      paintTexturedTile(drawn, map, layout, textures, index);
    }
    pixi.world.addChild(layer);
    return drawn;
  };
</script>

<script lang="ts">
  type HexMapRendererProps = {
    pixi: PixiContext;
    map: HexMapData;
    layout: HexLayout;
    /**
     * Row-major indices of the tiles whose terrain changed since the last paint, as a projection's
     * delta reports them. `null` repaints the whole map, which is also what a new `map` does.
     */
    dirtyTiles?: Uint32Array | null;
    /** Draws the flat colors of the terrains only, without the textures of their tiles. */
    flatColors?: boolean;
    /** Called when the pointer moves to another tile, with `null` once it leaves the map. */
    onTileHover?: (tile: OffsetCoords | null) => void;
    /** Called when a tile is clicked. */
    onTileClick?: (tile: OffsetCoords) => void;
  };
  const {
    pixi,
    map,
    layout,
    dirtyTiles = null,
    flatColors = false,
    onTileHover,
    onTileClick,
  }: HexMapRendererProps = $props();

  let textures = $state.raw<TerrainTextures | null>(null);
  onMount(() => {
    loadTerrainTextures().then(
      (loadedTextures) => (textures = loadedTextures),
      (error) => console.error("Failed to load the terrain textures:", error),
    );
  });

  let drawn = $state.raw<DrawnMap | null>(null);

  // the map is drawn whole when it is replaced, and when the textures finally load
  $effect(() => {
    const currentMap = map;
    const currentTextures = textures;
    const currentDrawn = drawMap(pixi, currentMap, layout, currentTextures);
    drawn = currentDrawn;

    const hoverOutline = new GraphicsContext().poly(hexCorners(layout)).stroke({ width: 3, color: 0xffffff });
    const hoverMarker = currentDrawn.layer.addChild(new Graphics({ context: hoverOutline, visible: false }));

    // the textured tiles fade in once hexes are large enough on screen for their details
    const updateTexturesOpacity = () => {
      const hexHeight = 2 * layout.size * pixi.world.scale.y;
      const fadeIn = (hexHeight - TEXTURES_FADE_IN.from) / (TEXTURES_FADE_IN.to - TEXTURES_FADE_IN.from);
      currentDrawn.texturedTiles.alpha = flatColors ? 0 : Math.min(Math.max(fadeIn, 0), 1);
      currentDrawn.texturedTiles.visible = currentDrawn.texturedTiles.alpha > 0;
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
      const hex = hexAt(layout, event.getLocalPosition(currentDrawn.layer));
      const tile = wrapHex(currentMap, hex);
      hoverMarker.visible = tile !== null;
      hoverMarker.position = hexCenter(layout, hex);
      hover(tile);
    };
    const onPointerLeave = () => {
      hoverMarker.visible = false;
      hover(null);
    };
    const onPointerTap = (event: FederatedPointerEvent) => {
      const tile = wrapHex(currentMap, hexAt(layout, event.getLocalPosition(currentDrawn.layer)));
      if (tile) {
        onTileClick?.(tile);
      }
    };
    pixi.world.on("pointermove", onPointerMove).on("pointerleave", onPointerLeave).on("pointertap", onPointerTap);

    return () => {
      pixi.app.ticker.remove(updateTexturesOpacity);
      pixi.world.off("pointermove", onPointerMove).off("pointerleave", onPointerLeave).off("pointertap", onPointerTap);
      drawn = null;
      // the textures are shared, cached by PixiJS's `Assets`
      currentDrawn.layer.destroy({ children: true });
      for (const context of currentDrawn.rows) {
        context.destroy();
      }
      hoverOutline.destroy();
    };
  });

  // and tile by tile when a delta says which ones moved: a unit stepping into the fog repaints two
  // rows, not the map
  $effect(() => {
    const changed = dirtyTiles;
    const currentDrawn = drawn;
    if (!currentDrawn || !changed) {
      return;
    }
    // the indices come up in ascending order, so a row is painted once; out of order, the worst
    // that happens is painting one twice
    let paintedRow = -1;
    for (const index of changed) {
      if (index >= map.rows * map.columns) {
        continue;
      }
      paintTexturedTile(currentDrawn, map, layout, textures, index);
      const row = Math.floor(index / map.columns);
      if (row !== paintedRow) {
        paintRow(currentDrawn, map, layout, row);
        paintedRow = row;
      }
    }
  });
</script>
