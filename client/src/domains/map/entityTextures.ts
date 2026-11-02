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

/** The frames found missing so far, so that each is reported once rather than on every redraw. */
const missing = new Set<string>();

/**
 * The frames a sprite is drawn from, the mark being `null` for an entity painted in nobody's colors.
 *
 * `null` when its sheet has no base frame for it: the drawing names art that is not there - for a
 * building, a script's sprite key with no PNG behind it - and is left undrawn rather than drawn as
 * something else. Reported once per frame. A missing mark is not an error: not every piece of art
 * has one.
 */
export const spriteTextures = (
  textures: EntityTextures,
  sprite: EntitySprite,
): { base: Texture; mark: Texture | null } | null => {
  const frames: Partial<Record<string, Texture>> = textures[sprite.entitySet];
  const base = frames[sprite.base];
  if (!base) {
    const frame = `${sprite.entitySet}/${sprite.base}`;
    if (!missing.has(frame)) {
      missing.add(frame);
      console.warn(`No frame "${sprite.base}" in the ${sprite.entitySet} sheet: it is not drawn.`);
    }
    return null;
  }
  return { base, mark: (sprite.mark && frames[sprite.mark]) || null };
};
