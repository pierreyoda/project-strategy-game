/**
 * Colors the nations are drawn in, assigned by `NationId`.
 *
 * Picked to stay apart from the terrain palette of `terrain.ts`, and from one another under the fog,
 * which washes everything towards the background.
 */
export const NATION_COLORS = [
  0x4f9ddb, // blue
  0xd4604f, // red
  0x6fbf73, // green
  0xd9a441, // amber
  0xa77bcf, // violet
  0x46b8ac, // teal
  0xd47fb0, // pink
  0x9aa5b1, // steel
] as const;

/** The color of a nation, wrapping around once there are more nations than colors. */
export const nationColor = (nation: number): number => NATION_COLORS[nation % NATION_COLORS.length];
