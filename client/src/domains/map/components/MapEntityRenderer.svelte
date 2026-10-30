<script lang="ts" module>
  import { Container, Graphics, GraphicsContext, Sprite, type Texture } from "pixi.js";
  import { onMount } from "svelte";

  import type { PixiContext } from "../../../shared/pixi";
  import { type MapEntity, entityCoords, inDrawOrder } from "../entities";
  import { type EntityTextures, loadEntityTextures, spriteTextures } from "../entityTextures";
  import { type HexLayout, hexCenter, wrapBounds } from "../hexLayout";
  import type { HexMapData } from "../hexMap";

  /**
   * Heights of a hex on screen, in pixels, over which the sprites fade in over the tokens.
   *
   * The same window as the tiles' own textures in `HexMapRenderer`: the map changes from a diagram
   * into a place all at once, rather than growing buildings over flat colors.
   */
  const SPRITES_FADE_IN = { from: 16, to: 32 };
  /** Over the map, under the fog: what a nation cannot see, it cannot see the settlements of. See `GameView`. */
  const ENTITIES_Z_INDEX = 5;
  /** Corner radius of a token, as a share of its side. */
  const TOKEN_CORNER = 0.2;

  /**
   * The drawn entities, as two layers showing one at a time.
   *
   * There is no addressing anything here: the list is rebuilt from the world's state and the layer
   * with it. Entities are sparse enough that this costs less than tracking which ones moved, which
   * is what the overlay already concluded for the markers these sprites replace.
   */
  type DrawnEntities = {
    layer: Container;
    tokens: Container;
    sprites: Container;
    /** Geometry of the tokens, shared by every entity drawn at the same size. */
    tokenShapes: Map<number, GraphicsContext>;
  };

  /** The token geometry for this side, centered on the origin, built once and reused. */
  const tokenShape = (drawn: DrawnEntities, layout: HexLayout, side: number): GraphicsContext => {
    const existing = drawn.tokenShapes.get(side);
    if (existing) {
      return existing;
    }
    const length = side * layout.size;
    const shape = new GraphicsContext()
      .roundRect(-length / 2, -length / 2, length, length, length * TOKEN_CORNER)
      .fill(0xffffff);
    drawn.tokenShapes.set(side, shape);
    return shape;
  };

  /** A frame of an entity's sprite, scaled to the hex it stands on. */
  const entitySprite = (layout: HexLayout, texture: Texture, tint?: number): Sprite => {
    const sprite = new Sprite({ texture, anchor: 0.5 });
    // after the texture, which sizes the sprite to the texture's own dimensions
    sprite.setSize(layout.width, 2 * layout.size);
    if (tint !== undefined) {
      sprite.tint = tint;
    }
    return sprite;
  };

  const drawEntities = (
    pixi: PixiContext,
    map: HexMapData,
    layout: HexLayout,
    textures: EntityTextures | null,
    entities: readonly MapEntity[],
  ): DrawnEntities => {
    const layer = new Container({ label: "map-entities", zIndex: ENTITIES_Z_INDEX });
    const tokens = layer.addChild(new Container({ label: "entity-tokens" }));
    const sprites = layer.addChild(new Container({ label: "entity-sprites", isRenderGroup: true }));
    const drawn: DrawnEntities = { layer, tokens, sprites, tokenShapes: new Map() };

    // drawn twice side by side to wrap around east-west, like the map under it
    const copiesX = [0, wrapBounds(layout, map).period];
    const ordered = inDrawOrder(entities);
    for (const x of copiesX) {
      const tokenCopy = tokens.addChild(new Container({ x }));
      const spriteCopy = sprites.addChild(new Container({ x }));
      // in draw order, so that a settlement overhanging its hex is overlapped by its southern neighbors
      for (const entity of ordered) {
        const position = hexCenter(layout, entityCoords(map.columns, entity.tile));
        tokenCopy.addChild(
          new Graphics({
            context: tokenShape(drawn, layout, entity.token.side),
            position,
            tint: entity.tint,
            alpha: entity.alpha,
          }),
        );
        if (!textures) {
          continue;
        }
        const { base, mark } = spriteTextures(textures, entity.sprite);
        const parts = spriteCopy.addChild(new Container({ position, alpha: entity.alpha }));
        parts.addChild(entitySprite(layout, base));
        if (mark) {
          // drawn white, so that tinting it gives the owner's color rather than a blend of the two
          parts.addChild(entitySprite(layout, mark, entity.tint));
        }
      }
    }

    pixi.world.addChild(layer);
    return drawn;
  };
</script>

<script lang="ts">
  type MapEntityRendererProps = {
    pixi: PixiContext;
    map: HexMapData;
    layout: HexLayout;
    /** What to draw on the tiles, with its colors already resolved. A new list redraws the layer. */
    entities: readonly MapEntity[];
  };
  const { pixi, map, layout, entities }: MapEntityRendererProps = $props();

  let textures = $state.raw<EntityTextures | null>(null);
  onMount(() => {
    loadEntityTextures().then(
      (loadedTextures) => (textures = loadedTextures),
      (error) => console.error("Failed to load the entity textures:", error),
    );
  });

  $effect(() => {
    const drawn = drawEntities(pixi, map, layout, textures, entities);

    // the sprites take over from the tokens once hexes are large enough on screen for their details
    const updateDetail = () => {
      const hexHeight = 2 * layout.size * pixi.world.scale.y;
      const fadeIn = (hexHeight - SPRITES_FADE_IN.from) / (SPRITES_FADE_IN.to - SPRITES_FADE_IN.from);
      const detail = Math.min(Math.max(fadeIn, 0), 1);
      drawn.sprites.alpha = detail;
      drawn.sprites.visible = detail > 0;
      drawn.tokens.alpha = 1 - detail;
      drawn.tokens.visible = detail < 1;
    };
    pixi.app.ticker.add(updateDetail);

    return () => {
      pixi.app.ticker.remove(updateDetail);
      // the textures are shared, cached by PixiJS's `Assets`
      drawn.layer.destroy({ children: true });
      for (const shape of drawn.tokenShapes.values()) {
        shape.destroy();
      }
    };
  });
</script>
