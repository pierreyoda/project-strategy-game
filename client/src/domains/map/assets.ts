import { Assets, type AssetsManifest } from "pixi.js";

/** Built by AssetPack into the public directory (see `vite.assetpack.ts`), relative to the page. */
const ASSETS_DIRECTORY = "./assets/";

let initialization: Promise<void> | undefined;

const initialize = async () => {
  const response = await fetch(`${ASSETS_DIRECTORY}manifest.json`);
  const manifest: AssetsManifest = await response.json();
  for (const bundle of manifest.bundles) {
    // AssetPack lists the assets of its bundles in arrays
    if (Array.isArray(bundle.assets)) {
      for (const asset of bundle.assets) {
        // everything on the map is drawn much smaller than its texture when zoomed out
        asset.data = { ...asset.data, textureOptions: { autoGenerateMipmaps: true } };
      }
    }
  }
  await Assets.init({
    manifest,
    basePath: ASSETS_DIRECTORY,
    // AssetPack builds the textures for high (@2x) and standard (@1x) density screens
    texturePreference: { resolution: window.devicePixelRatio > 1 ? [2, 1] : [1, 2], format: ["webp", "png"] },
  });
};

/**
 * Initialize PixiJS's `Assets` with AssetPack's manifest, once for the whole game.
 *
 * `Assets` is a singleton and refuses a second initialization, so every loader of a bundle goes
 * through this rather than initializing it for itself.
 */
export const initializeAssets = (): Promise<void> => (initialization ??= initialize());
