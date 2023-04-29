const importGameWasmPkg = async () => import("../../wasm-pkg");
type RawWasmBridgeModule = Omit<Awaited<ReturnType<typeof importGameWasmPkg>>, 'default'>;

export interface WasmBridgeApi {
  core: WasmBridgeCoreModule;
  procgen: WasmBridgeProcGenModule;
}

export interface WasmBridgeCoreModule {
  buildHello: RawWasmBridgeModule['core_build_hello'];
}
const bindCoreModule = (wasmModule: Readonly<RawWasmBridgeModule>): WasmBridgeCoreModule => ({
  buildHello: wasmModule.core_build_hello,
});

export interface WasmBridgeProcGenModule {
  buildHello: RawWasmBridgeModule['procgen_build_hello'];
}
const bindProcGenModule = (wasmModule: Readonly<RawWasmBridgeModule>): WasmBridgeProcGenModule => ({
  buildHello: wasmModule.procgen_build_hello,
});

/**
 * Dynamically loads, inits, and binds the Rust-WASM module into a friendly API.
 *
 * **NB**: should only be called once.
 */
export const linkedWasmBridgeApi = async (): Promise<WasmBridgeApi | null> => {
  // FIXME: tried many combinations, but the Rust-WASM functions end up undefined
  // getting the webpack setup to work with no errors was already an up-hill battle,
  // but not there is must be an issue with my env (macOS Ventura), wasm-bindgen, wasm-pack, the webpack wasm-pack plugin, Electron, or a mix of those
  try {
    const wasmModule = await importGameWasmPkg();
    return {
      core: bindCoreModule(wasmModule),
      procgen: bindProcGenModule(wasmModule),
    };
  } catch (err) {
    console.error(`WASM linking error:`, err);
    return null;
  }
};
