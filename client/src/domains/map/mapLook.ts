/**
 * The knobs of the map's look: what keeps the textured tiles from reading as wallpaper.
 *
 * One object rather than scattered constants so that the planet generator's test bench can turn
 * them live; the game itself only ever draws with `DEFAULT_MAP_LOOK`.
 */
export type MapLook = {
  /**
   * How far a tile's tint strays from white, as a share of full brightness: the same tile over and
   * over reads as wallpaper, and a few percent apart they read as ground. A tint can only darken, so
   * this takes every channel down together...
   */
  tintBrightness: number;
  /** ...and this takes each channel down on its own, which drifts the tile slightly warm or cool. */
  tintHue: number;
  /** How far the darkest patch of the ground noise goes under full brightness. */
  noiseDepth: number;
  /** The side of one repetition of the ground noise, in world units: 400 is about seven hexes of `GameView`'s layout. */
  noiseWorldSize: number;
  /** Share of the tiles of a terrain picked among its decorated tiles, when it has some. */
  decoratedShare: number;
};

export const DEFAULT_MAP_LOOK: MapLook = {
  tintBrightness: 0.05,
  tintHue: 0.015,
  noiseDepth: 0.35,
  noiseWorldSize: 750,
  decoratedShare: 0.1,
};
