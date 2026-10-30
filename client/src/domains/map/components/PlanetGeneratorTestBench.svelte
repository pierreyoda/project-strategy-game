<script lang="ts">
  import { onDestroy, onMount } from "svelte";

  import Button from "../../../shared/ui/Button.svelte";
  import GameView from "../../game/ui/GameView.svelte";
  import { createFakeEntities } from "../fakeEntities";
  import { type HexMapData, type OffsetCoords, tileIndex } from "../hexMap";
  import {
    DEFAULT_OCEAN_COVERAGE,
    MAX_MAP_SIZE,
    MAX_SEED,
    type PlanetSettings,
    createPlanetGenerator,
    validatePlanetSettings,
  } from "../procgen/planetGenerator";
  import { TERRAIN_NAMES, type TerrainKind } from "../terrain";

  const generator = createPlanetGenerator();
  onDestroy(() => generator.dispose());

  const settings = $state<PlanetSettings>({
    seed: 0,
    columns: 80,
    rows: 52,
    oceanCoverage: DEFAULT_OCEAN_COVERAGE,
  });
  const settingsError = $derived(validatePlanetSettings(settings));

  let map = $state.raw<HexMapData | null>(null);
  let progress = $state<number | null>(null);
  let duration = $state<number | null>(null);
  let error = $state<string | null>(null);
  let hoveredTile = $state<OffsetCoords | null>(null);
  const hoveredTileIndex = $derived(hoveredTile && map && tileIndex(map, hoveredTile));
  let flatColors = $state(false);
  let showSettlements = $state(false);
  // stand-ins, to see the sprites on real terrain: the generator has no settlements to place
  const entities = $derived(map && showSettlements ? createFakeEntities(map) : []);

  // generating cancels the generation in progress: only the latest one updates the state
  let generation = 0;
  const generate = async () => {
    const id = ++generation;
    progress = 0;
    error = null;
    const start = performance.now();
    try {
      const generated = await generator.generate($state.snapshot(settings), (percentage) => {
        if (id === generation) {
          progress = percentage;
        }
      });
      if (id === generation) {
        map = generated;
        hoveredTile = null;
        duration = performance.now() - start;
      }
    } catch (generationError) {
      if (id === generation) {
        error = generationError instanceof Error ? generationError.message : String(generationError);
      }
    } finally {
      if (id === generation) {
        progress = null;
      }
    }
  };

  const generateWithRandomSeed = () => {
    settings.seed = Math.floor(Math.random() * (MAX_SEED + 1));
    void generate();
  };

  onMount(() => void generate());
</script>

{#snippet numberField(label: string, value: number, max: number, onValue: (value: number) => void)}
  <label class="flex items-center gap-2 text-fg-muted">
    {label}
    <input
      type="number"
      min="0"
      {max}
      {value}
      oninput={(event) => onValue(event.currentTarget.valueAsNumber)}
      class="h-7 w-24 rounded-tl-cut-sm rounded-br-cut-sm border border-line bg-surface-sunken px-2 text-fg tabular-nums chamfer focus-visible:outline-2 focus-visible:outline-accent"
    />
  </label>
{/snippet}

<div class="flex h-screen flex-col">
  <header class="flex items-center gap-6 border-b border-line bg-surface px-4 py-2 text-sm">
    <h1 class="font-display text-lg font-semibold tracking-wide whitespace-nowrap">Planet generator</h1>
    <form
      class="flex items-center gap-3"
      onsubmit={(event) => {
        event.preventDefault();
        void generate();
      }}
    >
      {@render numberField("Seed", settings.seed, MAX_SEED, (seed) => (settings.seed = seed))}
      {@render numberField("Columns", settings.columns, MAX_MAP_SIZE, (columns) => (settings.columns = columns))}
      {@render numberField("Rows", settings.rows, MAX_MAP_SIZE, (rows) => (settings.rows = rows))}
      {@render numberField(
        "Ocean %",
        Math.round(settings.oceanCoverage * 100),
        100,
        (percentage) => (settings.oceanCoverage = percentage / 100),
      )}
      <Button type="submit" variant="primary" size="sm" icon="map" disabled={settingsError !== null}>Generate</Button>
      <Button size="sm" onclick={generateWithRandomSeed}>Random seed</Button>
    </form>
    <Button variant="ghost" size="sm" aria-pressed={flatColors} onclick={() => (flatColors = !flatColors)}>
      Flat colors
    </Button>
    <Button
      variant="ghost"
      size="sm"
      aria-pressed={showSettlements}
      onclick={() => (showSettlements = !showSettlements)}
    >
      Settlements
    </Button>

    <div class="ml-auto flex items-center gap-4 whitespace-nowrap text-fg-muted tabular-nums">
      {#if settingsError}
        <span class="text-warning">{settingsError}</span>
      {:else if error}
        <span class="text-danger">{error}</span>
      {/if}
      {#if progress !== null}
        <div class="flex items-center gap-2">
          <div class="h-1.5 w-32 bg-surface-sunken">
            <div class="h-full bg-accent" style:width="{progress}%"></div>
          </div>
          {progress}%
        </div>
      {:else if duration !== null}
        <span>Generated in {(duration / 1000).toFixed(2)} s</span>
      {/if}
      {#if map && hoveredTile && hoveredTileIndex !== null}
        <span class="text-fg">
          ({hoveredTile.col}, {hoveredTile.row}) · {TERRAIN_NAMES[map.terrain[hoveredTileIndex] as TerrainKind]} ·
          {map.elevations[hoveredTileIndex]} m
        </span>
      {/if}
    </div>
  </header>
  <div class="min-h-0 flex-1">
    {#if map}
      <GameView {map} {entities} {flatColors} onTileHover={(tile) => (hoveredTile = tile)} />
    {/if}
  </div>
</div>
