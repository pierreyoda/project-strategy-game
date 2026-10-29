const importGameWasmPkg = async () => import("../../wasm-pkg");
type RawWasmBridgeModule = Omit<Awaited<ReturnType<typeof importGameWasmPkg>>, "default">;

export interface WasmBridgeApi {
  core: WasmBridgeCoreModule;
}

export interface WasmBridgeCoreModule {
}
const bindCoreModule = (wasmModule: Readonly<RawWasmBridgeModule>): WasmBridgeCoreModule => ({
});

/**
 * Dynamically loads, inits, and binds the Rust-WASM module into a friendly API.
 *
 * **NB**: should only be called once.
 */
export const linkedWasmBridgeApi = async (): Promise<WasmBridgeApi | null> => {
  try {
    const wasmModule = await importGameWasmPkg();
    return {
      core: bindCoreModule(wasmModule),
    };
  } catch (err) {
    console.error(`WASM linking error:`, err);
    return null;
  }
};
