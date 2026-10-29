import { Assets, type AssetsManifest, type Spritesheet, type Texture } from "pixi.js";

import type { TerrainKind } from "./terrain";
import { TERRAIN_TILES, TERRAIN_TILE_SETS, type TileSet } from "./tileset";

/** Built by AssetPack into the public directory (see `vite.assetpack.ts`), relative to the page. */
const ASSETS_DIRECTORY = "./assets/";

export type TerrainTextures = Record<TerrainKind, { base: Texture[]; decorated: Texture[] }>;

let initialization: Promise<void> | undefined;

const initializeAssets = async () => {
  const response = await fetch(`${ASSETS_DIRECTORY}manifest.json`);
  const manifest: AssetsManifest = await response.json();
  for (const bundle of manifest.bundles) {
    // AssetPack lists the assets of its bundles in arrays
    if (TERRAIN_TILE_SETS.includes(bundle.name as TileSet) && Array.isArray(bundle.assets)) {
      for (const asset of bundle.assets) {
        // tiles are drawn much smaller than their textures when zoomed out
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

/** Load the textures of the terrains' tiles, from the sprite sheets of their tile sets. */
export const loadTerrainTextures = async (): Promise<TerrainTextures> => {
  initialization ??= initializeAssets();
  await initialization;
  await Assets.loadBundle(TERRAIN_TILE_SETS);

  const entries = Object.entries(TERRAIN_TILES).map(([kind, { tileSet, base, decorated }]) => {
    const { textures } = Assets.get<Spritesheet>(`tiles/${tileSet}`);
    const frameTextures = (frames: string[]) => frames.map((frame) => textures[frame]);
    return [kind, { base: frameTextures(base), decorated: frameTextures(decorated) }];
  });
  return Object.fromEntries(entries);
};
