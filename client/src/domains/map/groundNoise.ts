import { CanvasSource, Texture } from "pixi.js";

/**
 * Large, soft patches of lighter and darker ground, multiplied over the textured tiles.
 *
 * A per-tile jitter breaks repetition between neighbors; this breaks it across a region, where a
 * dozen hexes of the same grass would otherwise read as flat. It is generated rather than shipped:
 * one texture of a few kilobytes, periodic so that it tiles. How deep and how wide are `MapLook`'s.
 */

/** Side of the generated texture, in pixels: the noise is only ever magnified, so it need not be large. */
export const GROUND_NOISE_SIZE = 256;

/** The noise is the same at every start: a planet should not change its shade on reload. */
const SEED = 0x9e3779b9;

/** Pseudo-random number in [0, 1), always the same for the given lattice point (hashed with "lowbias32"). */
const latticeRandom = (x: number, y: number, octave: number): number => {
  let hash = Math.imul(x, 0x27d4eb2d) ^ Math.imul(y, 0x165667b1) ^ Math.imul(octave, 0x9e3779b1) ^ SEED;
  hash = Math.imul(hash ^ (hash >>> 16), 0x7feb352d);
  hash = Math.imul(hash ^ (hash >>> 15), 0x846ca68b);
  return ((hash ^ (hash >>> 16)) >>> 0) / 2 ** 32;
};

const smoothstep = (t: number) => t * t * (3 - 2 * t);

/**
 * Value noise in [0, 1) at the given pixel, periodic over `GROUND_NOISE_SIZE`.
 *
 * Each octave interpolates a lattice whose period divides the texture's size, so every octave wraps
 * and so does their sum.
 */
const noiseAt = (x: number, y: number): number => {
  // lattice points per side, and the weight of that octave: coarse patches first, then detail
  const octaves: [number, number][] = [
    [4, 1],
    [8, 0.5],
    [16, 0.25],
  ];
  let total = 0;
  let weights = 0;
  for (const [points, weight] of octaves) {
    const cell = GROUND_NOISE_SIZE / points;
    const [fx, fy] = [x / cell, y / cell];
    const [x0, y0] = [Math.floor(fx), Math.floor(fy)];
    const [tx, ty] = [smoothstep(fx - x0), smoothstep(fy - y0)];
    // wrapping the lattice is what makes the texture tile
    const corner = (dx: number, dy: number) => latticeRandom((x0 + dx) % points, (y0 + dy) % points, points);
    const top = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * tx;
    const bottom = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * tx;
    total += (top + (bottom - top) * ty) * weight;
    weights += weight;
  }
  return total / weights;
};

/** Draw the noise into a texture that repeats, grey from full brightness down to `depth` under it. */
export const createGroundNoiseTexture = (depth: number): Texture => {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = GROUND_NOISE_SIZE;
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("the ground noise needs a 2D canvas context.");
  }
  const image = context.createImageData(GROUND_NOISE_SIZE, GROUND_NOISE_SIZE);
  for (let y = 0; y < GROUND_NOISE_SIZE; y++) {
    for (let x = 0; x < GROUND_NOISE_SIZE; x++) {
      const brightness = Math.round(255 * (1 - depth * noiseAt(x, y)));
      const offset = 4 * (y * GROUND_NOISE_SIZE + x);
      image.data[offset] = image.data[offset + 1] = image.data[offset + 2] = brightness;
      image.data[offset + 3] = 255;
    }
  }
  context.putImageData(image, 0, 0);
  return new Texture({ source: new CanvasSource({ resource: canvas, addressMode: "repeat" }) });
};
