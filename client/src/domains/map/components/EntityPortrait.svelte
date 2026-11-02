<script lang="ts" module>
  import type { Texture } from "pixi.js";

  import type { EntitySprite } from "../entities";
  import { type EntityTextures, loadEntityTextures, spriteTextures } from "../entityTextures";

  /** The textures, loaded once for every portrait: `Assets` caches them, this spares the promise. */
  let loading: Promise<EntityTextures> | undefined;

  /**
   * Draw a sprite sheet's frame at the place it held in its untrimmed image, scaled to `width` ×
   * `height`: the sheet's units are its pixels over its resolution, and trimming cut the empty
   * margins off the frame.
   */
  const drawFrame = (
    context: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D,
    texture: Texture,
    width: number,
    height: number,
  ) => {
    const source = texture.source.resource as Parameters<CanvasRenderingContext2D["drawImage"]>[0];
    const resolution = texture.source.resolution;
    const { frame, orig, trim } = texture;
    const scale = { x: width / orig.width, y: height / orig.height };
    const placed = trim ?? { x: 0, y: 0, width: orig.width, height: orig.height };
    context.drawImage(
      source,
      frame.x * resolution,
      frame.y * resolution,
      frame.width * resolution,
      frame.height * resolution,
      placed.x * scale.x,
      placed.y * scale.y,
      placed.width * scale.x,
      placed.height * scale.y,
    );
  };
</script>

<script lang="ts">
  type EntityPortraitProps = {
    sprite: EntitySprite;
    /** The owner's colour, for the mark. */
    tint: number;
    /** Size on the page, in CSS pixels: the art's own 1110 × 1280 proportions are the caller's to keep. */
    width: number;
    height: number;
    /** Faded, for a building still going up. */
    ghost?: boolean;
    class?: string;
  };
  const { sprite, tint, width, height, ghost = false, class: className }: EntityPortraitProps = $props();

  let canvas: HTMLCanvasElement;
  let textures = $state.raw<EntityTextures | null>(null);

  $effect(() => {
    loading ??= loadEntityTextures();
    loading.then(
      (loaded) => (textures = loaded),
      (error) => console.error("Failed to load the entity textures:", error),
    );
  });

  // the art of the map's sprites, in the page: the frames are cut from the same sheets the map draws
  $effect(() => {
    const context = canvas.getContext("2d");
    if (!context || !textures) {
      return;
    }
    const ratio = window.devicePixelRatio;
    const [pixelWidth, pixelHeight] = [Math.round(width * ratio), Math.round(height * ratio)];
    canvas.width = pixelWidth;
    canvas.height = pixelHeight;
    context.clearRect(0, 0, pixelWidth, pixelHeight);

    const frames = spriteTextures(textures, sprite);
    if (!frames) {
      // no art for it, which `spriteTextures` has reported already
      return;
    }
    const { base, mark } = frames;
    if (base.rotate !== 0 || (mark && mark.rotate !== 0)) {
      // AssetPack packs its sheets unrotated: a rotated frame would need the whole transform, for nothing
      console.warn(`No portrait for "${sprite.base}": its frame is rotated in the sheet.`);
      return;
    }
    drawFrame(context, base, pixelWidth, pixelHeight);
    if (mark) {
      // the mark is drawn white: filled with the tint where it stands, then laid over the base
      const marked = new OffscreenCanvas(pixelWidth, pixelHeight);
      const markContext = marked.getContext("2d")!;
      drawFrame(markContext, mark, pixelWidth, pixelHeight);
      markContext.globalCompositeOperation = "source-in";
      markContext.fillStyle = `#${tint.toString(16).padStart(6, "0")}`;
      markContext.fillRect(0, 0, pixelWidth, pixelHeight);
      context.drawImage(marked, 0, 0);
    }
  });
</script>

<canvas
  bind:this={canvas}
  class={["block", ghost && "opacity-35 saturate-[.4]", className]}
  style:width="{width}px"
  style:height="{height}px"
  aria-hidden="true"
></canvas>
