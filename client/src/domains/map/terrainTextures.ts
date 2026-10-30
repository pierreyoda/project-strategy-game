import { Assets, type Spritesheet, type Texture } from "pixi.js";

import { initializeAssets } from "./assets";
import type { TerrainKind } from "./terrain";
import { TERRAIN_TILES, TERRAIN_TILE_SETS } from "./tileset";

export type TerrainTextures = Record<TerrainKind, { base: readonly Texture[]; decorated: readonly Texture[] }>;

/** Load the textures of the terrains' tiles, from the sprite sheets of their tile sets. */
export const loadTerrainTextures = async (): Promise<TerrainTextures> => {
  await initializeAssets();
  await Assets.loadBundle(TERRAIN_TILE_SETS);

  const entries = Object.entries(TERRAIN_TILES).map(([kind, { tileSet, base, decorated }]) => {
    const { textures } = Assets.get<Spritesheet>(`tiles/${tileSet}`);
    const frameTextures = (frames: string[]) => frames.map((frame) => textures[frame]);
    return [kind, { base: frameTextures(base), decorated: frameTextures(decorated) }];
  });
  return Object.fromEntries(entries);
};
