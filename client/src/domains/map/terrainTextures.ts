import { Assets, type Spritesheet, type Texture } from "pixi.js";

import { initializeAssets } from "./assets";
import type { TerrainKind } from "./terrain";
import { FEATURES_BUNDLE, TERRAIN_TILES } from "./tileset";

export type TerrainTextures = Record<TerrainKind, { base: readonly Texture[]; decorated: readonly Texture[] }>;

/** Load the textures of the terrains' tiles, from the sprite sheets of their tile sets. */
export const loadTerrainTextures = async (): Promise<TerrainTextures> => {
  await initializeAssets();
  await Assets.loadBundle(FEATURES_BUNDLE);

  const { textures } = Assets.get<Spritesheet>(FEATURES_BUNDLE);
  const entries = Object.entries(TERRAIN_TILES).map(([kind, { base, decorated }]) => {
    const frameTextures = (frames: string[]) => frames.map((frame) => textures[frame]);
    return [kind, { base: frameTextures(base), decorated: frameTextures(decorated) }];
  });
  return Object.fromEntries(entries);
};
