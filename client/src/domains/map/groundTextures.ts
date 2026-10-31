import { Assets, type Texture } from "pixi.js";

import { initializeAssets } from "./assets";
import { TerrainKind } from "./terrain";

/**
 * Bundle built by AssetPack from `raw-assets/ground/terrains{m}/`: one loose, tileable image per
 * terrain. Loose on purpose - packed in a sprite sheet, a texture could not repeat.
 */
const GROUND_BUNDLE = "terrains";

/** The image of each terrain's ground, by the name it was derived under (see `scripts/ground-from-tiles.ts`). */
const GROUND_IMAGES: Record<TerrainKind, string> = {
  [TerrainKind.DeepWater]: "deep_water",
  [TerrainKind.ShallowWater]: "shallow_water",
  [TerrainKind.Flat]: "flat",
  [TerrainKind.Hills]: "hills",
  [TerrainKind.Mountains]: "mountains",
  [TerrainKind.Ice]: "ice",
  [TerrainKind.RoughIce]: "rough_ice",
};

export type GroundTextures = Record<TerrainKind, Texture>;

/** Load the ground texture of every terrain, set to repeat: the ground shader tiles them over the map. */
export const loadGroundTextures = async (): Promise<GroundTextures> => {
  await initializeAssets();
  const loaded: Record<string, Texture> | undefined = await Assets.loadBundle(GROUND_BUNDLE);
  // `Assets` resolves to nothing rather than failing for a bundle the manifest does not list
  if (!loaded) {
    throw new Error(`the assets manifest lists no "${GROUND_BUNDLE}" bundle: were the ground textures built?`);
  }

  const entries = Object.entries(GROUND_IMAGES).map(([kind, name]) => {
    // AssetPack aliases a loose image by its file name, with and without the bundle's path
    const file = `${name}.png`;
    const texture = Object.entries(loaded).find(([alias]) => alias === file || alias.endsWith(`/${file}`))?.[1];
    if (!texture) {
      throw new Error(`the ground bundle holds no "${name}" image: ${Object.keys(loaded).join(", ")}`);
    }
    texture.source.addressMode = "repeat";
    return [kind, texture];
  });
  return Object.fromEntries(entries);
};
