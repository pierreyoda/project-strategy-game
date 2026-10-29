import type { HexMapData } from "./hexMap";
import { TerrainKind } from "./terrain";

/**
 * Deterministic stand-in for a generated planet, wrapping around seamlessly east-west, to work on the map
 * rendering without the procedural generation.
 *
 * The default size is the one of `MapGeneratorSettings` in the `procgen` crate.
 */
export const createFakeHexMap = (seed: number, columns = 80, rows = 52): HexMapData => {
  const elevations = new Int16Array(columns * rows);
  const terrain = new Uint8Array(columns * rows);
  // like `TerrainClassifier` in the `procgen` crate
  const polarRows = Math.max(1, Math.round(rows * 0.05));
  for (let row = 0; row < rows; row++) {
    const latitude = (Math.PI * row) / (rows - 1);
    for (let col = 0; col < columns; col++) {
      // periodic in the longitude, odd rows being shoved half a hex to the east
      const longitude = (2 * Math.PI * (col + 0.5 * (row & 1))) / columns;
      const value =
        Math.sin(3 * longitude + seed) * Math.sin(2 * latitude + 0.7 * seed) +
        0.5 * Math.sin(7 * longitude - 4 * latitude + 1.3 * seed);
      const elevation = Math.round(3000 * value);

      const index = row * columns + col;
      elevations[index] = elevation;
      if (row < polarRows || row >= rows - polarRows) {
        terrain[index] = elevation < 1024 ? TerrainKind.Ice : TerrainKind.RoughIce;
      } else if (elevation < -128) {
        terrain[index] = TerrainKind.DeepWater;
      } else if (elevation < 0) {
        terrain[index] = TerrainKind.ShallowWater;
      } else if (elevation < 1024) {
        terrain[index] = TerrainKind.Flat;
      } else if (elevation < 2048) {
        terrain[index] = TerrainKind.Hills;
      } else {
        terrain[index] = TerrainKind.Mountains;
      }
    }
  }
  return { columns, rows, elevations, terrain };
};
