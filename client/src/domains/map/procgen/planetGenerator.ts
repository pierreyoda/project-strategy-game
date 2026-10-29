import type { HexMapData } from "../hexMap";

/** Highest supported seed, like `MAX_SEED` in the `procgen` crate. */
export const MAX_SEED = 0xffff_ffff - 256;
/** Bounds of the number of columns and of rows: `procgen` stores them as `i16`. */
export const MAX_MAP_SIZE = 1024;
/** Share of the map covered by the seas, like `ocean_coverage` in `MapGeneratorSettings::default`. */
export const DEFAULT_OCEAN_COVERAGE = 0.7;

export type PlanetSettings = {
  seed: number;
  columns: number;
  rows: number;
  /** Share of the map covered by the seas, from 0 (no sea) to 1 (no land). */
  oceanCoverage: number;
};

/** Message from the main thread to `planet.worker.ts`. */
export type PlanetGeneratorRequest = {
  settings: PlanetSettings;
};

/** Messages from `planet.worker.ts` to the main thread. */
export type PlanetGeneratorResponse =
  { type: "progress"; percentage: number } | { type: "done"; map: HexMapData } | { type: "error"; message: string };

/** Why the given settings cannot generate a planet, if they cannot. */
export const validatePlanetSettings = ({ seed, columns, rows, oceanCoverage }: PlanetSettings): string | null => {
  if (!Number.isInteger(seed) || seed < 0 || seed > MAX_SEED) {
    return `The seed must be an integer from 0 to ${MAX_SEED}.`;
  }
  const isValidSize = (size: number) => Number.isInteger(size) && size >= 1 && size <= MAX_MAP_SIZE;
  if (!isValidSize(columns) || !isValidSize(rows)) {
    return `The columns and the rows must be integers from 1 to ${MAX_MAP_SIZE}.`;
  }
  if (!Number.isFinite(oceanCoverage) || oceanCoverage < 0 || oceanCoverage > 1) {
    return "The ocean coverage must be a share from 0 to 1.";
  }
  return null;
};

export type PlanetGenerator = {
  /**
   * Generate a planet, cancelling the generation in progress if any: its promise is then rejected with
   * an `AbortError`.
   */
  generate: (settings: PlanetSettings, onProgress?: (percentage: number) => void) => Promise<HexMapData>;
  /** Cancel the generation in progress if any, and release the worker. */
  dispose: () => void;
};

/**
 * Generates planets with the WASM bridge in a Web Worker, so as not to block the UI.
 *
 * The generation is a single call to the WASM bridge, which cannot be interrupted: cancelling it means
 * terminating the worker, and spawning another one for the next generation.
 */
export const createPlanetGenerator = (): PlanetGenerator => {
  let worker: Worker | null = null;
  let cancelGeneration: (() => void) | null = null;

  const cancel = () => {
    cancelGeneration?.();
    cancelGeneration = null;
  };

  return {
    generate: (settings, onProgress) => {
      cancel();
      const error = validatePlanetSettings(settings);
      if (error) {
        return Promise.reject(new RangeError(error));
      }

      const generationWorker = (worker ??= new Worker(new URL("./planet.worker.ts", import.meta.url), {
        type: "module",
      }));
      return new Promise((resolve, reject) => {
        const settle = () => {
          cancelGeneration = null;
          generationWorker.onmessage = null;
          generationWorker.onerror = null;
        };
        cancelGeneration = () => {
          settle();
          generationWorker.terminate();
          worker = null;
          reject(new DOMException("The planet generation was cancelled.", "AbortError"));
        };
        generationWorker.onmessage = ({ data: response }: MessageEvent<PlanetGeneratorResponse>) => {
          if (response.type === "progress") {
            onProgress?.(response.percentage);
            return;
          }
          settle();
          if (response.type === "done") {
            resolve(response.map);
          } else {
            reject(new Error(response.message));
          }
        };
        // the worker itself failed, for instance to load the WASM bridge: it cannot be reused
        generationWorker.onerror = (event) => {
          event.preventDefault();
          settle();
          generationWorker.terminate();
          worker = null;
          reject(new Error(`The planet generator failed: ${event.message}`));
        };
        generationWorker.postMessage({ settings } satisfies PlanetGeneratorRequest);
      });
    },
    dispose: () => {
      cancel();
      worker?.terminate();
      worker = null;
    },
  };
};
