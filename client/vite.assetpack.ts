import path from "node:path";
import { fileURLToPath } from "node:url";

import { AssetPack, type AssetPackConfig } from "@assetpack/core";
import { pixiPipes } from "@assetpack/core/pixi";
import type { Plugin } from "vite";

// independent of Vite's root, which Storybook sets differently
const CLIENT_DIR = path.dirname(fileURLToPath(import.meta.url));

const ASSETPACK_CONFIG: AssetPackConfig = {
  entry: path.join(CLIENT_DIR, "raw-assets"),
  output: path.join(CLIENT_DIR, "public/assets"),
  cacheLocation: path.join(CLIENT_DIR, ".assetpack"),
  pipes: pixiPipes({
    // the raw assets are drawn for high density screens
    resolutions: { high: 2, default: 1 },
    // mipmaps average blocks of pixels: wider gaps keep the textures of a sprite sheet from bleeding into each other
    texturePacker: { texturePacker: { padding: 8 } },
  }),
};

/**
 * Builds the game's assets from `raw-assets/` with AssetPack, into Vite's public directory along with their
 * manifest for PixiJS's `Assets`: once when building, or watching them when serving.
 */
export function assetPack(): Plugin {
  let isServing = false;
  let watcher: AssetPack | null = null;

  return {
    name: "project-x:assetpack",
    configResolved(config) {
      isServing = config.command === "serve";
    },
    async buildStart() {
      if (!isServing) {
        await new AssetPack(ASSETPACK_CONFIG).run();
      } else if (!watcher) {
        watcher = new AssetPack(ASSETPACK_CONFIG);
        await watcher.watch();
      }
    },
    async buildEnd() {
      await watcher?.stop();
      watcher = null;
    },
  };
}
