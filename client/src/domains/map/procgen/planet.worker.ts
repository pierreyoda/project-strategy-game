import type { HexMapData } from "../hexMap";
import type { PlanetGeneratorRequest, PlanetGeneratorResponse } from "./planetGenerator";

// Imported lazily: instantiating the WASM bridge relies on top-level await, during which a module worker
// already receives messages, before a static import would let `onmessage` be set.
const bridge = import("../../../../../wasm-pkg");

const respond = (response: PlanetGeneratorResponse, transfer: Transferable[] = []) =>
  postMessage(response, { transfer });

onmessage = async ({ data: { settings } }: MessageEvent<PlanetGeneratorRequest>) => {
  try {
    const { procgen_generate_planet } = await bridge;
    const planet = procgen_generate_planet(
      settings.seed,
      settings.columns,
      settings.rows,
      settings.oceanCoverage,
      (percentage) => respond({ type: "progress", percentage }),
    );
    let map: HexMapData;
    try {
      map = { columns: planet.columns, rows: planet.rows, elevations: planet.elevations, terrain: planet.terrain };
    } finally {
      planet.free();
    }
    // moved to the main thread rather than copied
    respond({ type: "done", map }, [map.elevations.buffer, map.terrain.buffer]);
  } catch (error) {
    respond({ type: "error", message: error instanceof Error ? error.message : String(error) });
  }
};
