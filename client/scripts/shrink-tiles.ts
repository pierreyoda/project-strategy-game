/**
 * Shrinks the original art of the map tiles (1110×1280 px) to the 512 px tall masters packed by AssetPack
 * (see `vite.assetpack.ts`): 512 px is the most a hex is displayed on, on a high density screen at full zoom.
 *
 * Usage: `node scripts/shrink-tiles.ts <originals directory> [<output directory>]`, the originals directory holding
 * a directory of PNG files per tile set (`water/`, `nature/`...).
 */
import { mkdir, readdir } from "node:fs/promises";
import path from "node:path";

import sharp from "sharp";

const TILE_HEIGHT = 512;

const [originalsDir, outputDir = "raw-assets/tiles"] = process.argv.slice(2);
if (!originalsDir) {
  console.error("Usage: node scripts/shrink-tiles.ts <originals directory> [<output directory>]");
  process.exit(1);
}

for (const tileSet of await readdir(originalsDir, { withFileTypes: true })) {
  if (!tileSet.isDirectory()) {
    continue;
  }
  // AssetPack tags: each tile set is a bundle (`{m}`) packed in a sprite sheet (`{tps}`)
  const tileSetOutputDir = path.join(outputDir, `${tileSet.name}{m}{tps}`);
  await mkdir(tileSetOutputDir, { recursive: true });

  const files = (await readdir(path.join(originalsDir, tileSet.name))).filter(
    (file) => path.extname(file).toLowerCase() === ".png",
  );
  for (const file of files) {
    // lower case extensions: AssetPack's texture packer ignores `.PNG` files
    const output = path.join(tileSetOutputDir, `${path.parse(file).name}.png`);
    await sharp(path.join(originalsDir, tileSet.name, file))
      .resize({ height: TILE_HEIGHT })
      .png()
      .toFile(output);
  }
  console.log(`${tileSet.name}: ${files.length} tiles`);
}
