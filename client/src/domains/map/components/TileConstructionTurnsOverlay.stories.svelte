<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";

  import type { MapConstruction } from "../constructions";
  import { type MapEntity, SETTLEMENT_SPRITES, SETTLEMENT_TOKENS, SettlementStage } from "../entities";
  import type { HexMapData } from "../hexMap";
  import { TerrainKind } from "../terrain";
  import MapEntityRenderer from "./MapEntityRenderer.svelte";
  import MapStoryStage from "./MapStoryStage.svelte";
  import TileConstructionTurnsOverlay from "./TileConstructionTurnsOverlay.svelte";

  // no `component`: every story draws its own stage, which svelte-csf would otherwise nest in the overlay
  const { Story } = defineMeta({
    title: "Map/TileConstructionTurnsOverlay",
    parameters: { layout: "centered" },
    argTypes: {
      turnsLeft: { control: { type: "range", min: 1, max: 20, step: 1 } },
      turnsTotal: { control: { type: "range", min: 1, max: 20, step: 1 } },
    },
  });

  /** A patch of flat land with a few hills, the same every time: stories should not change between reloads. */
  const patch = (columns: number, rows: number, hills: readonly number[] = []): HexMapData => {
    const terrain = new Uint8Array(columns * rows).fill(TerrainKind.Flat);
    for (const tile of hills) {
      terrain[tile] = TerrainKind.Hills;
    }
    return { columns, rows, terrain, elevations: new Int16Array(columns * rows) };
  };

  const at = (map: HexMapData, col: number, row: number) => row * map.columns + col;

  const construction = (id: number, tile: number, turnsLeft: number, turnsTotal: number): MapConstruction => ({
    id,
    tile,
    turnsLeft,
    turnsTotal,
  });

  // five stages of five constructions, from just started to its last turn
  const row = patch(7, 3, [0, 13, 15]);
  const progress = [
    construction(1, at(row, 1, 1), 5, 5),
    construction(2, at(row, 2, 1), 4, 5),
    construction(3, at(row, 3, 1), 3, 6),
    construction(4, at(row, 4, 1), 12, 20),
    construction(5, at(row, 5, 1), 1, 4),
  ];

  const single = patch(3, 3, [0, 8]);
  const zooms = [0.5, 1, 2, 3];

  const town = patch(5, 3, [9]);
  const settlement = (id: number, tile: number): MapEntity => ({
    id,
    tile,
    sprite: SETTLEMENT_SPRITES[SettlementStage.Town],
    token: SETTLEMENT_TOKENS[SettlementStage.Town],
    tint: 0x4f9ddb,
  });
  const towns = [settlement(1, at(town, 1, 1)), settlement(2, at(town, 3, 1))];
</script>

<!-- From just started to its last turn: the border fills clockwise from the top, the last turn's number in the accent. -->
<Story name="Progress">
  <MapStoryStage map={row} zoom={1.5}>
    {#snippet children(pixi, layout)}
      <TileConstructionTurnsOverlay {pixi} map={row} {layout} constructions={progress} />
    {/snippet}
  </MapStoryStage>
</Story>

<!-- Turn the knobs: the overlay redraws as the construction's turns change. -->
<Story name="Single tile" args={{ turnsLeft: 3, turnsTotal: 5 }}>
  {#snippet template(args)}
    <MapStoryStage map={single} zoom={3}>
      {#snippet children(pixi, layout)}
        <TileConstructionTurnsOverlay
          {pixi}
          map={single}
          {layout}
          constructions={[construction(1, at(single, 1, 1), args.turnsLeft ?? 3, args.turnsTotal ?? 5)]}
        />
      {/snippet}
    </MapStoryStage>
  {/snippet}
</Story>

<!-- The same construction at four zooms: under ×1 the label leaves first, then the number; the plate and border stay. -->
<Story name="Zoom levels">
  <div class="flex flex-wrap items-end gap-8">
    {#each zooms as zoom (zoom)}
      <figure class="m-0 flex flex-col items-center gap-2">
        <MapStoryStage map={single} {zoom}>
          {#snippet children(pixi, layout)}
            <TileConstructionTurnsOverlay
              {pixi}
              map={single}
              {layout}
              constructions={[construction(1, at(single, 1, 1), 3, 5)]}
            />
          {/snippet}
        </MapStoryStage>
        <figcaption class="font-display text-xs font-semibold tracking-widest text-fg-subtle uppercase">
          ×{zoom} · hex {64 * zoom} px
        </figcaption>
      </figure>
    {/each}
  </div>
</Story>

<!-- Over the building it is raising: the plate dims the settlement on the right, the left one is finished. -->
<Story name="Over a settlement">
  <MapStoryStage map={town} zoom={2}>
    {#snippet children(pixi, layout)}
      <MapEntityRenderer {pixi} map={town} {layout} entities={towns} />
      <TileConstructionTurnsOverlay
        {pixi}
        map={town}
        {layout}
        constructions={[construction(1, at(town, 3, 1), 2, 6)]}
      />
    {/snippet}
  </MapStoryStage>
</Story>
