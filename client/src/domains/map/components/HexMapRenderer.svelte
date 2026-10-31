<script lang="ts" module>
  import {
    BufferImageSource,
    Container,
    type FederatedPointerEvent,
    Geometry,
    Graphics,
    GraphicsContext,
    Mesh,
    type Shader,
    Sprite,
    Texture,
    TilingSprite,
  } from "pixi.js";
  import { onMount } from "svelte";

  import type { PixiContext } from "../../../shared/pixi";
  import { GROUND_NOISE_SIZE, createGroundNoiseTexture } from "../groundNoise";
  import { createGroundShader, groundUniforms } from "../groundShader";
  import { type GroundTextures, loadGroundTextures } from "../groundTextures";
  import { type HexLayout, hexAt, hexCenter, hexCorners, mapHeight, wrapBounds, wrapHex } from "../hexLayout";
  import type { HexMapData, OffsetCoords } from "../hexMap";
  import { DEFAULT_MAP_LOOK, type MapLook } from "../mapLook";
  import { TERRAIN_COLORS, isExplored } from "../terrain";
  import { type TerrainTextures, loadTerrainTextures } from "../terrainTextures";
  import { TERRAIN_TILES, pickTileFlip, pickTileIsDecorated, pickTileTint, pickTileVariant } from "../tileset";

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
    /** The ground noise over the textured tiles, generated for this map and destroyed with it. */
    noise: Texture;
    /**
     * The blended ground under the tiles, once its textures are loaded: one mesh over the whole map,
     * reading the terrains straight from the map's own buffer.
     */
    ground: { mesh: Mesh<Geometry, Shader>; geometry: Geometry; shader: Shader; source: BufferImageSource } | null;
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
    look: MapLook,
    index: number,
  ) => {
    const terrain = map.terrain[index];
    const coords = { col: index % map.columns, row: Math.floor(index / map.columns) };
    const explored = textures !== null && isExplored(terrain);
    // the ground layer paints the ground: a tile with nothing standing on it has nothing left to draw
    const visible =
      explored && (TERRAIN_TILES[terrain].feature || pickTileIsDecorated(textures[terrain], coords, look));
    for (const sprites of drawn.tileSprites) {
      const sprite = sprites[index];
      sprite.visible = visible;
      if (!visible) {
        continue;
      }
      sprite.texture = pickTileVariant(textures[terrain], coords, look);
      // after the texture, which resets the sprite to the texture's own size...
      sprite.setSize(layout.width * TEXTURED_TILE_SCALE, 2 * layout.size * TEXTURED_TILE_SCALE);
      // ...and leaves the scale positive, which the flips are the sign of
      const flip = pickTileFlip(terrain, coords);
      sprite.scale.x = Math.abs(sprite.scale.x) * (flip.x ? -1 : 1);
      sprite.scale.y = Math.abs(sprite.scale.y) * (flip.y ? -1 : 1);
      sprite.tint = pickTileTint(coords, look);
    }
  };

  const drawMap = (
    pixi: PixiContext,
    map: HexMapData,
    layout: HexLayout,
    textures: TerrainTextures | null,
    grounds: GroundTextures | null,
    look: MapLook,
  ): DrawnMap => {
    const layer = new Container({ label: "hex-map", zIndex: MAP_Z_INDEX });
    // drawn twice side by side to wrap around east-west, see `wrapBounds`
    const { period } = wrapBounds(layout, map);
    const copiesX = [0, period];

    const rows = Array.from({ length: map.rows }, () => new GraphicsContext());
    for (const x of copiesX) {
      const copy = layer.addChild(new Container({ label: "flat-tiles", x }));
      for (const context of rows) {
        copy.addChild(new Graphics({ context }));
      }
    }

    // over the flat colors, which also fill any gap between the textured tiles
    const texturedTiles = layer.addChild(new Container({ label: "textured-tiles", isRenderGroup: true }));

    // the ground first, under the tiles that still stand on it
    let ground: DrawnMap["ground"] = null;
    if (grounds) {
      const source = new BufferImageSource({
        resource: map.terrain,
        width: map.columns,
        height: map.rows,
        format: "r8unorm",
        scaleMode: "nearest",
        alphaMode: "no-premultiply-alpha",
      });
      const shader = createGroundShader(source, grounds, groundUniforms(map, layout, period, look));
      // the odd rows of the second copy stick out past it by half a hex
      const [right, bottom] = [2 * period + layout.width, mapHeight(layout, map)];
      const geometry = new Geometry({
        attributes: { aPosition: new Float32Array([0, 0, right, 0, right, bottom, 0, bottom]) },
        indexBuffer: new Uint32Array([0, 1, 2, 0, 2, 3]),
      });
      const mesh = texturedTiles.addChild(new Mesh({ label: "ground", geometry, shader }));
      ground = { mesh, geometry, shader, source };
    }

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

    // the noise's own period has to divide the map's, or the second copy would not match the first
    const noise = createGroundNoiseTexture(look.noiseDepth);
    const noiseWorldSize = period / Math.max(1, Math.round(period / look.noiseWorldSize));
    texturedTiles.addChild(
      new TilingSprite({
        label: "ground-noise",
        texture: noise,
        // the odd rows of the second copy stick out past it by half a hex
        width: 2 * period + layout.width,
        height: mapHeight(layout, map),
        tileScale: { x: noiseWorldSize / GROUND_NOISE_SIZE, y: noiseWorldSize / GROUND_NOISE_SIZE },
        blendMode: "multiply",
      }),
    );

    const drawn: DrawnMap = { layer, rows, texturedTiles, tileSprites, noise, ground };
    for (let row = 0; row < map.rows; row++) {
      paintRow(drawn, map, layout, row);
    }
    for (let index = 0; index < map.rows * map.columns; index++) {
      paintTexturedTile(drawn, map, layout, textures, look, index);
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
    /** The knobs of the textured tiles' variety. A new one redraws the map whole. */
    look?: MapLook;
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
    look = DEFAULT_MAP_LOOK,
    onTileHover,
    onTileClick,
  }: HexMapRendererProps = $props();

  let textures = $state.raw<TerrainTextures | null>(null);
  let grounds = $state.raw<GroundTextures | null>(null);
  onMount(() => {
    loadTerrainTextures().then(
      (loadedTextures) => (textures = loadedTextures),
      (error) => console.error("Failed to load the terrain textures:", error),
    );
    loadGroundTextures().then(
      (loadedGrounds) => (grounds = loadedGrounds),
      (error) => console.error("Failed to load the ground textures:", error),
    );
  });

  let drawn = $state.raw<DrawnMap | null>(null);

  // the map is drawn whole when it is replaced, and when the textures finally load
  $effect(() => {
    const currentMap = map;
    const currentTextures = textures;
    const currentDrawn = drawMap(pixi, currentMap, layout, currentTextures, grounds, look);
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
      // the textures are shared, cached by PixiJS's `Assets`; the noise is this map's own
      currentDrawn.layer.destroy({ children: true });
      currentDrawn.noise.destroy(true);
      if (currentDrawn.ground) {
        // the mesh went with the layer; what it drew with is this map's own
        currentDrawn.ground.shader.destroy();
        currentDrawn.ground.geometry.destroy();
        currentDrawn.ground.source.destroy();
      }
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
      paintTexturedTile(currentDrawn, map, layout, textures, look, index);
      const row = Math.floor(index / map.columns);
      if (row !== paintedRow) {
        paintRow(currentDrawn, map, layout, row);
        paintedRow = row;
      }
    }
    // the ground reads the map's buffer, patched in place by the delta: it only has to be told
    currentDrawn.ground?.source.update();
  });
</script>
