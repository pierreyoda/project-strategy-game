/** How the map steps back under a lens, whichever layer it is. */

import { ColorMatrixFilter } from "pixi.js";

/**
 * How far the ground steps back under a lens, as on the "Resources" artboard: down to grey, at 42% of
 * its brightness, so that whatever the lens brings forward reads over it. A trace of colour is left,
 * for water to still read as water.
 */
const MUTED_SATURATION = -0.85;
const MUTED_BRIGHTNESS = 0.42;

/** A filter muting the layer it is set on, for a lens. */
export const createMutedFilter = (): ColorMatrixFilter => {
  // at the resolution of the screen, or the map would blur on high density screens
  const filter = new ColorMatrixFilter({ resolution: "inherit" });
  filter.saturate(MUTED_SATURATION);
  filter.brightness(MUTED_BRIGHTNESS, true);
  return filter;
};

/** Opacity of what stands on the map - settlements, buildings, units - while a lens looks past it. */
export const DIMMED_ALPHA = 0.4;
