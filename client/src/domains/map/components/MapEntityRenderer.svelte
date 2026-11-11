<script lang="ts" module>
  import { Container, Graphics, GraphicsContext, Sprite, type Texture } from "pixi.js";
  import { onMount } from "svelte";

  import type { PixiContext } from "../../../shared/pixi";
  import {
    type CompanyState,
    type EntityStrength,
    type MapEntity,
    companyRows,
    companyStates,
    entityCoords,
    inDrawOrder,
  } from "../entities";
  import { type EntityTextures, loadEntityTextures, spriteTextures } from "../entityTextures";
  import { type HexLayout, hexCenter, wrapBounds } from "../hexLayout";
  import type { HexMapData } from "../hexMap";
  import { DIMMED_ALPHA } from "../lensLook";

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
  /** The sprites' frame, in which a counter's card ends at y 835 (see the Units artboard). */
  const SPRITE_FRAME = { width: 1110, height: 1280 };
  /**
   * A counter's companies, in the sprites' frame: circles of `radius` centred on the card's bottom
   * edge, half on the card and half over the ground, `gap` apart, the lines wrapping `lineGap` apart.
   */
  const COMPANY_ROW = { y: 835, radius: 46, gap: 12, lineGap: 8 };
  /** The design system's tokens a company is drawn with - `--color-ground`, `--color-line-strong`, `--color-warning` - and its light. */
  const COMPANY_COLORS = { disc: 0x090e14, rim: 0x405670, warning: 0xf0b43c, light: 0xf6fbff, glow: 0xbfe9ff };

  /** The dark disc every company sits on, at a radius of 1, rimmed like the HUD's panels. */
  const disc = (shape: GraphicsContext) =>
    shape.circle(0, 0, 1).fill(COMPANY_COLORS.disc).stroke({ width: 0.065, color: COMPANY_COLORS.rim });
  /**
   * A company's circle at a radius of 1, to scale: every one sits on a dark disc of its own, so that
   * it reads on any ground. Active, a white core in a soft glow; suppressed, a ring in the warning
   * colour; lost, a faint socket, so that the unit's full size still shows.
   */
  const COMPANY_SHAPES: Record<CompanyState, () => GraphicsContext> = {
    active: () => {
      const shape = new GraphicsContext();
      // the glow, as rings fading out rather than a filter on every counter
      for (const [radius, alpha] of [
        [1.3, 0.08],
        [1.2, 0.14],
        [1.1, 0.22],
      ]) {
        shape.circle(0, 0, radius).fill({ color: COMPANY_COLORS.glow, alpha });
      }
      return disc(shape)
        .circle(0, 0, 0.78)
        .fill(COMPANY_COLORS.light)
        .circle(0, 0, 0.78)
        .stroke({ width: 0.12, color: COMPANY_COLORS.glow, alpha: 0.7 });
    },
    suppressed: () =>
      disc(new GraphicsContext()).circle(0, 0, 0.65).stroke({ width: 0.22, color: COMPANY_COLORS.warning }),
    lost: () => disc(new GraphicsContext()).circle(0, 0, 0.72).fill({ color: 0xffffff, alpha: 0.1 }),
  };
  /** Opacity of a sprite with nothing left to do this turn. */
  const SPENT_ALPHA = 0.55;
  /** A building still going up: fainter than a spent unit, like the panels' portrait of one. */
  const GHOST_ALPHA = 0.4;

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
    /** Geometry of a company, by state, shared by every counter. */
    companyShapes: Record<CompanyState, GraphicsContext>;
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

  /**
   * The strength row of a counter, laid out in the sprite's frame and scaled with it: the sprite is a
   * frame fitted to the hex, so a frame coordinate maps to the hex the same way for both.
   */
  const strengthRow = (drawn: DrawnEntities, layout: HexLayout, strength: EntityStrength): Container => {
    const sx = layout.width / SPRITE_FRAME.width;
    const sy = (2 * layout.size) / SPRITE_FRAME.height;
    const pitch = 2 * COMPANY_ROW.radius + COMPANY_ROW.gap;
    const row = new Container();
    companyRows(companyStates(strength)).forEach((line, index) => {
      const left = -((line.length - 1) * pitch) / 2;
      const y = COMPANY_ROW.y + index * (2 * COMPANY_ROW.radius + COMPANY_ROW.lineGap) - SPRITE_FRAME.height / 2;
      line.forEach((state, company) => {
        row.addChild(
          new Graphics({
            context: drawn.companyShapes[state],
            position: { x: (left + company * pitch) * sx, y: y * sy },
            scale: COMPANY_ROW.radius * sx,
          }),
        );
      });
    });
    return row;
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
    const drawn: DrawnEntities = {
      layer,
      tokens,
      sprites,
      tokenShapes: new Map(),
      companyShapes: {
        active: COMPANY_SHAPES.active(),
        suppressed: COMPANY_SHAPES.suppressed(),
        lost: COMPANY_SHAPES.lost(),
      },
    };

    // drawn twice side by side to wrap around east-west, like the map under it
    const copiesX = [0, wrapBounds(layout, map).period];
    const ordered = inDrawOrder(entities);
    for (const x of copiesX) {
      const tokenCopy = tokens.addChild(new Container({ x }));
      const spriteCopy = sprites.addChild(new Container({ x }));
      // in draw order, so that a settlement overhanging its hex is overlapped by its southern neighbors
      for (const entity of ordered) {
        const position = hexCenter(layout, entityCoords(map.columns, entity.tile));
        if (entity.token) {
          tokenCopy.addChild(
            new Graphics({
              context: tokenShape(drawn, layout, entity.token.side),
              position,
              tint: entity.tint,
            }),
          );
        }
        if (!textures) {
          continue;
        }
        const frames = spriteTextures(textures, entity.sprite);
        if (!frames) {
          // no art for it: its token still says whose it is, zoomed out
          continue;
        }
        const { base, mark } = frames;
        const parts = spriteCopy.addChild(new Container({ position }));
        parts.addChild(entitySprite(layout, base));
        if (mark) {
          // drawn white, so that tinting it gives the owner's color rather than a blend of the two
          parts.addChild(entitySprite(layout, mark, entity.tint));
        }
        if (entity.strength) {
          parts.addChild(strengthRow(drawn, layout, entity.strength));
        }
        if (entity.ghost) {
          parts.alpha = GHOST_ALPHA;
        } else if (entity.spent) {
          parts.alpha = SPENT_ALPHA;
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
    /** Steps everything back, for a lens to bring something else forward. */
    dimmed?: boolean;
  };
  const { pixi, map, layout, entities, dimmed = false }: MapEntityRendererProps = $props();

  let layer = $state.raw<Container | null>(null);
  $effect(() => {
    if (layer) {
      layer.alpha = dimmed ? DIMMED_ALPHA : 1;
    }
  });

  let textures = $state.raw<EntityTextures | null>(null);
  onMount(() => {
    loadEntityTextures().then(
      (loadedTextures) => (textures = loadedTextures),
      (error) => console.error("Failed to load the entity textures:", error),
    );
  });

  $effect(() => {
    const drawn = drawEntities(pixi, map, layout, textures, entities);
    layer = drawn.layer;

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
      layer = null;
      // the textures are shared, cached by PixiJS's `Assets`
      drawn.layer.destroy({ children: true });
      for (const shape of [...drawn.tokenShapes.values(), ...Object.values(drawn.companyShapes)]) {
        shape.destroy();
      }
    };
  });
</script>
