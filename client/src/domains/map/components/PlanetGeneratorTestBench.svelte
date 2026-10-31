<script lang="ts">
  import { onDestroy, onMount } from "svelte";

  import Button from "../../../shared/ui/Button.svelte";
  import Panel from "../../../shared/ui/Panel.svelte";
  import GameView from "../../game/ui/GameView.svelte";
  import { createFakeEntities } from "../fakeEntities";
  import { NO_NATION } from "../../game/session/protocol";
  import { type HexMapData, type OffsetCoords, tileIndex } from "../hexMap";
  import { DEFAULT_MAP_LOOK, type MapLook } from "../mapLook";
  import {
    DEFAULT_NATIONS,
    DEFAULT_OCEAN_COVERAGE,
    MAX_MAP_SIZE,
    MAX_NATIONS,
    MAX_SEED,
    type PlanetNations,
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
    nations: DEFAULT_NATIONS,
  });
  const settingsError = $derived(validatePlanetSettings(settings));

  let map = $state.raw<HexMapData | null>(null);
  let nations = $state.raw<PlanetNations | null>(null);
  let showNations = $state(true);
  let progress = $state<number | null>(null);
  let duration = $state<number | null>(null);
  let error = $state<string | null>(null);
  let hoveredTile = $state<OffsetCoords | null>(null);
  const hoveredTileIndex = $derived(hoveredTile && map && tileIndex(map, hoveredTile));
  let flatColors = $state(false);
  let showSettlements = $state(false);
  let showLook = $state(false);
  // the knobs of the tiles' variety, to tune by eye before they are baked into `DEFAULT_MAP_LOOK`
  const look = $state<MapLook>({ ...DEFAULT_MAP_LOOK });
  const lookChanged = $derived(
    Object.keys(DEFAULT_MAP_LOOK).some((key) => look[key as keyof MapLook] !== DEFAULT_MAP_LOOK[key as keyof MapLook]),
  );
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
        map = generated.map;
        nations = generated.nations;
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

{#snippet lookSlider(label: string, key: keyof MapLook, min: number, max: number, step: number)}
  <label class="grid grid-cols-[7rem_1fr_3.5rem] items-center gap-2 text-fg-muted">
    {label}
    <input type="range" {min} {max} {step} bind:value={look[key]} class="accent-accent" />
    <span class="text-right text-fg tabular-nums">{look[key]}</span>
  </label>
{/snippet}

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
      {@render numberField("Nations", settings.nations, MAX_NATIONS, (count) => (settings.nations = count))}
      <Button type="submit" variant="primary" size="sm" icon="map" disabled={settingsError !== null}>Generate</Button>
      <Button size="sm" onclick={generateWithRandomSeed}>Random seed</Button>
    </form>
    <Button variant="ghost" size="sm" aria-pressed={flatColors} onclick={() => (flatColors = !flatColors)}>
      Flat colors
    </Button>
    <Button variant="ghost" size="sm" aria-pressed={showNations} onclick={() => (showNations = !showNations)}>
      Nations
    </Button>
    <Button
      variant="ghost"
      size="sm"
      aria-pressed={showSettlements}
      onclick={() => (showSettlements = !showSettlements)}
    >
      Settlements
    </Button>
    <Button variant="ghost" size="sm" aria-pressed={showLook} onclick={() => (showLook = !showLook)}>Look</Button>

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
      {#if nations && progress === null}
        <span>
          Spacing {nations.report.spacingRequested} → {nations.report.spacingFinal}
          {#if nations.report.unplaced > 0}· {nations.report.unplaced} unplaced{/if}
          · {nations.report.unclaimedLand} hexes unclaimed
        </span>
      {/if}
      {#if map && hoveredTile && hoveredTileIndex !== null}
        <span class="text-fg">
          ({hoveredTile.col}, {hoveredTile.row}) · {TERRAIN_NAMES[map.terrain[hoveredTileIndex] as TerrainKind]} ·
          {map.elevations[hoveredTileIndex]} m
          {#if nations && nations.tileOwner[hoveredTileIndex] !== NO_NATION}
            · Nation {nations.tileOwner[hoveredTileIndex] + 1}
          {:else if nations && nations.tileSphere[hoveredTileIndex] !== NO_NATION}
            · sphere of Nation {nations.tileSphere[hoveredTileIndex] + 1}
          {/if}
        </span>
      {/if}
    </div>
  </header>
  <div class="relative min-h-0 flex-1">
    {#if map}
      <GameView
        {map}
        {entities}
        nations={showNations ? nations : null}
        {flatColors}
        {look}
        onTileHover={(tile) => (hoveredTile = tile)}
      />
    {/if}
    {#if showLook}
      <Panel title="Look" variant="glass" class="absolute right-3 bottom-3 w-96 text-sm">
        {#snippet actions()}
          <Button
            size="sm"
            variant="ghost"
            disabled={!lookChanged}
            onclick={() => Object.assign(look, DEFAULT_MAP_LOOK)}
          >
            Reset
          </Button>
        {/snippet}
        <div class="flex flex-col gap-2">
          {@render lookSlider("Tint brightness", "tintBrightness", 0, 0.15, 0.005)}
          {@render lookSlider("Tint hue", "tintHue", 0, 0.05, 0.005)}
          {@render lookSlider("Noise depth", "noiseDepth", 0, 0.4, 0.01)}
          {@render lookSlider("Noise size", "noiseWorldSize", 100, 1200, 25)}
          {@render lookSlider("Decorated share", "decoratedShare", 0, 1, 0.05)}
          {@render lookSlider("Edge band", "edgeBand", 0, 40, 1)}
          {@render lookSlider("Edge warp", "edgeWarp", 0, 30, 1)}
          {@render lookSlider("Edge warp scale", "edgeWarpScale", 20, 400, 10)}
          {@render lookSlider("Ground scale", "groundScale", 30, 400, 5)}
        </div>
      </Panel>
    {/if}
  </div>
</div>
