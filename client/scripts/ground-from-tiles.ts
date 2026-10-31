/**
 * Derives a seamless ground texture per terrain from the map tiles, for the ground shader to tile over the
 * map (see `groundShader.ts`): the middle of a tile's hex, made to repeat.
 *
 * The seam is hidden the classic way: the crop is blended with three copies of itself shifted by half its
 * size along x, along y, and along both, each weighted by a window that is zero exactly on that copy's own
 * seams - the crop's at its edges, the shifted ones' along the middle lines. Wherever one copy breaks,
 * another carries the pixel, so the result wraps. The price is some ghosting, which is why the source tiles
 * are the plainest of their terrain.
 *
 * Usage: `node scripts/ground-from-tiles.ts [<contact sheets directory>]`, the optional directory receiving a
 * 2×2 tiling of each texture, to judge the seam by eye.
 */
import { mkdir } from "node:fs/promises";
import path from "node:path";

import sharp from "sharp";

const TILES_DIR = "raw-assets/tiles";
/** AssetPack tags: a bundle (`{m}`) of loose images, which a sprite sheet (`{tps}`) could not let repeat. */
const OUTPUT_DIR = "raw-assets/ground/terrains{m}";
/** The textures are only ever magnified, so there is no point past this. */
const MAX_SIZE = 512;
/** Side of the crop, as a share of the tile's width: inside the hex's inscribed circle, clear of the painted rim. */
const CROP_SHARE = 0.58;
/** Sharpness of the blend windows: higher keeps each copy to itself longer, for less ghosting but harder joins. */
const WINDOW_POWER = 2;

/** The plainest tile of each terrain, by the name the ground shader loads the texture under. */
const SOURCES: Record<string, string> = {
  deep_water: "water{m}{tps}/water_tile_05.png",
  shallow_water: "water{m}{tps}/water_tile_03.png",
  flat: "nature{m}{tps}/nature_tile_01.png",
  hills: "nature{m}{tps}/nature_tile_20.png",
  mountains: "nature{m}{tps}/nature_tile_04.png",
  ice: "ice{m}{tps}/ice_tile_01.png",
  rough_ice: "ice{m}{tps}/ice_tile_05.png",
};

/** 0 at the ends of [0, 1] and 1 in the middle: the weight of a copy whose seams lie at the ends. */
const window = (t: number) => Math.sin(Math.PI * t) ** (2 * WINDOW_POWER);

const makeSeamless = async (source: string): Promise<{ data: Buffer; side: number }> => {
  const { width = 0, height = 0 } = await sharp(source).metadata();
  const side = Math.round(width * CROP_SHARE);
  const { data } = await sharp(source)
    .extract({ left: Math.round((width - side) / 2), top: Math.round((height - side) / 2), width: side, height: side })
    .removeAlpha()
    .raw()
    .toBuffer({ resolveWithObject: true });

  const half = Math.floor(side / 2);
  const output = Buffer.alloc(data.length);
  const pixel = (x: number, y: number) => 3 * ((y % side) * side + (x % side));
  for (let y = 0; y < side; y++) {
    for (let x = 0; x < side; x++) {
      // the crop's own window peaks in the middle; a copy shifted along an axis peaks at that axis's edges
      const [u, v] = [x / side, y / side];
      const [sideU, middleU, sideV, middleV] = [window(u), window(u + 0.5), window(v), window(v + 0.5)];
      const copies: [number, number][] = [
        [sideU * sideV, pixel(x, y)],
        [middleU * sideV, pixel(x + half, y)],
        [sideU * middleV, pixel(x, y + half)],
        [middleU * middleV, pixel(x + half, y + half)],
      ];
      const total = copies.reduce((sum, [weight]) => sum + weight, 0);
      const at = 3 * (y * side + x);
      for (let channel = 0; channel < 3; channel++) {
        const value = copies.reduce((sum, [weight, from]) => sum + weight * data[from + channel], 0);
        output[at + channel] = Math.round(value / total);
      }
    }
  }
  return { data: output, side };
};

const [sheetsDir] = process.argv.slice(2);
await mkdir(OUTPUT_DIR, { recursive: true });
if (sheetsDir) {
  await mkdir(sheetsDir, { recursive: true });
}

for (const [name, tile] of Object.entries(SOURCES)) {
  const { data, side } = await makeSeamless(path.join(TILES_DIR, tile));
  let image = sharp(data, { raw: { width: side, height: side, channels: 3 } });
  if (side > MAX_SIZE) {
    image = image.resize(MAX_SIZE, MAX_SIZE, { kernel: "lanczos3" });
  }
  const texture = await image.png().toBuffer();
  const output = path.join(OUTPUT_DIR, `${name}.png`);
  await sharp(texture).toFile(output);

  if (sheetsDir) {
    const size = Math.min(side, MAX_SIZE);
    await sharp({ create: { width: 2 * size, height: 2 * size, channels: 3, background: "#000" } })
      .composite([0, 1, 2, 3].map((i) => ({ input: texture, left: (i % 2) * size, top: Math.floor(i / 2) * size })))
      .png()
      .toFile(path.join(sheetsDir, `${name}.png`));
  }
  console.log(`${name}: ${Math.min(side, MAX_SIZE)} px from ${tile}`);
}
