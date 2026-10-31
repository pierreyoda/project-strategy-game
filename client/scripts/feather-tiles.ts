/**
 * Feathers the map tiles that still draw over the blended ground - the ones with something standing on
 * them, see `feature` in `tileset.ts` - so that their hex edge melts into it instead of cutting it.
 *
 * The alpha is multiplied by the distance to the hex's edge, ramping up over the outer `FEATHER` share of
 * the apothem. The frames are packed into one `features` bundle, whatever tile set they came from: the
 * ground layer draws every pure-ground tile now, so these are all that is left to load.
 *
 * Usage: `node scripts/feather-tiles.ts`, reading `raw-assets/tiles/` (see `shrink-tiles.ts`).
 */
import { mkdir, rm } from "node:fs/promises";
import path from "node:path";

import sharp from "sharp";

const TILES_DIR = "raw-assets/tiles";
/** AssetPack tags: one bundle (`{m}`), packed in a sprite sheet (`{tps}`). */
const OUTPUT_DIR = "raw-assets/features{m}{tps}";
/** Share of the apothem over which the edge fades in. */
const FEATHER = 0.12;

/**
 * The frames with a feature on them, by tile set: the base tiles of the terrains whose `feature` is set
 * and the decorated tiles of the others. Kept in step with `TERRAIN_TILES` in `tileset.ts` by hand - the
 * script runs under Node, where the client's modules are out of reach.
 */
const FEATURE_FRAMES: Record<string, string[]> = {
  water: ["water_tile_02.png"],
  nature: [
    // flat, decorated
    "nature_tile_03.png",
    "nature_tile_08.png",
    "nature_tile_10.png",
    "nature_tile_11.png",
    "nature_tile_22.png",
    // hills
    "nature_tile_06.png",
    "nature_tile_13.png",
    "nature_tile_16.png",
    "nature_tile_21.png",
    // mountains
    "nature_tile_04.png",
    "nature_tile_05.png",
    "nature_tile_14.png",
  ],
  ice: [
    // rough ice
    "ice_tile_05.png",
    "ice_tile_07.png",
    "ice_tile_08.png",
    "ice_tile_11.png",
    "ice_tile_13.png",
    "ice_tile_16.png",
  ],
};

const smoothstep = (t: number) => {
  const x = Math.min(Math.max(t, 0), 1);
  return x * x * (3 - 2 * x);
};

/**
 * Distance from a point to the edge of the pointy-top hex filling a `width` × `height` canvas, negative
 * outside: the lesser of the distances to its flat sides and to its slanted ones.
 */
const hexEdgeDistance = (x: number, y: number, width: number, height: number): number => {
  const [apothem, size] = [width / 2, height / 2];
  const [dx, dy] = [Math.abs(x - apothem), Math.abs(y - size)];
  return Math.min(apothem - dx, (1 - dx / (2 * apothem) - dy / size) * apothem);
};

await rm(OUTPUT_DIR, { recursive: true, force: true });
await mkdir(OUTPUT_DIR, { recursive: true });

let count = 0;
for (const [tileSet, frames] of Object.entries(FEATURE_FRAMES)) {
  for (const frame of frames) {
    const source = path.join(TILES_DIR, `${tileSet}{m}{tps}`, frame);
    const { data, info } = await sharp(source).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
    const feather = FEATHER * (info.width / 2);
    for (let y = 0; y < info.height; y++) {
      for (let x = 0; x < info.width; x++) {
        const at = 4 * (y * info.width + x) + 3;
        data[at] = Math.round(
          data[at] * smoothstep(hexEdgeDistance(x + 0.5, y + 0.5, info.width, info.height) / feather),
        );
      }
    }
    await sharp(data, { raw: { width: info.width, height: info.height, channels: 4 } })
      .png()
      .toFile(path.join(OUTPUT_DIR, frame));
    count++;
  }
}
console.log(`${count} feature tiles feathered into ${OUTPUT_DIR}`);
