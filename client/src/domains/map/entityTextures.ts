import { Assets, type Spritesheet, type Texture } from "pixi.js";

import { initializeAssets } from "./assets";
import { ENTITY_SETS, type EntitySet, type EntitySprite } from "./entities";

/** The frames of every entity set, by set and by frame name. */
export type EntityTextures = Record<EntitySet, Record<string, Texture>>;

/** Load the textures of the entities, from the sprite sheets of their entity sets. */
export const loadEntityTextures = async (): Promise<EntityTextures> => {
  await initializeAssets();
  await Assets.loadBundle(ENTITY_SETS);

  const entries = ENTITY_SETS.map((entitySet) => {
    const { textures } = Assets.get<Spritesheet>(`entities/${entitySet}`);
    return [entitySet, textures];
  });
  return Object.fromEntries(entries);
};

/** The frames a sprite is drawn from, the mark being `null` for an entity painted in nobody's colors. */
export const spriteTextures = (
  textures: EntityTextures,
  sprite: EntitySprite,
): { base: Texture; mark: Texture | null } => ({
  base: textures[sprite.entitySet][sprite.base],
  mark: sprite.mark ? textures[sprite.entitySet][sprite.mark] : null,
});
