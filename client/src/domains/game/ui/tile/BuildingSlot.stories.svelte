<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import type { ComponentProps } from "svelte";

  import { BuildingState } from "../../session/protocol";
  import type { TileBuilding } from "../../state/tileDetails";
  import BuildingSlot from "./BuildingSlot.svelte";
  import { FIGURES, RIVAL, YOU } from "./tileSamples";

  /** A building as the panels get it, with nothing under way unless the story says otherwise. */
  const building = (
    type: string,
    name: string,
    level: number,
    overrides: Partial<TileBuilding> = {},
  ): TileBuilding => ({
    id: 0,
    template: type,
    type,
    name,
    level,
    sprite: `${type}_${Math.max(level, 1)}`,
    levels: 3,
    state: 0,
    turnsLeft: 0,
    turnsTotal: 0,
    ...overrides,
  });

  const { Story } = defineMeta({
    title: "Game/Tile/BuildingSlot",
    component: BuildingSlot,
    tags: ["autodocs"],
    // every action wired, as the design shows them: the game leaves out those it has no order for
    args: {
      tint: YOU.color,
      figures: FIGURES,
      onBuild: () => {},
      onUpgrade: () => {},
      onCancel: () => {},
      onDemolish: () => {},
    },
    parameters: { layout: "centered" },
    decorators: [],
  });
</script>

{#snippet frame(args: ComponentProps<typeof BuildingSlot>)}
  <div class="w-89 border border-line bg-surface p-3.5">
    <BuildingSlot {...args} />
  </div>
{/snippet}

<Story name="Free" args={{ slot: { state: "free" } }} template={frame} />

<Story
  name="Under construction"
  args={{
    slot: {
      state: "construction",
      building: building("mine", "Mine", 0, { state: BuildingState.UnderConstruction, turnsLeft: 2, turnsTotal: 3 }),
      turnsLeft: 2,
      turnsTotal: 3,
    },
  }}
  template={frame}
/>

<Story name="Built" args={{ slot: { state: "built", building: building("mine", "Mine", 2) } }} template={frame} />

<Story
  name="Upgrading"
  args={{
    slot: {
      state: "built",
      building: building("mine", "Mine", 2, { state: BuildingState.Upgrading, turnsLeft: 4, turnsTotal: 5 }),
    },
  }}
  template={frame}
/>

<Story
  name="Halted"
  args={{
    slot: { state: "built", building: building("mine", "Mine", 2, { state: BuildingState.Halted }) },
  }}
  template={frame}
/>

<Story
  name="Built, highest level"
  args={{
    slot: { state: "built", building: building("solar_plant", "Solar power plant", 3) },
    figures: { ...FIGURES, upgrade: undefined },
  }}
  template={frame}
/>

<Story
  name="Another nation's"
  args={{
    slot: { state: "foreign", owner: RIVAL, building: building("solar_plant", "Solar power plant", 1) },
  }}
  template={frame}
/>

<Story name="No slot" args={{ slot: { state: "none", reason: "Nothing can be built on water." } }} template={frame} />

<!-- The nation's own land, but no district laid out on it: as `slotOf` words it -->
<Story
  name="No district"
  args={{
    slot: {
      state: "none",
      reason: "No district is laid out here: without one, only an extractor on its deposit can be built.",
    },
  }}
  template={frame}
/>

<Story name="Settlement" args={{ slot: { state: "settlement", settlement: "Town" } }} template={frame} />

<!-- While a turn resolves: the orders are the next turn's, so every action says why it waits. -->
<Story
  name="Actions unavailable"
  args={{
    slot: { state: "built", building: building("mine", "Mine", 2) },
    unavailable: "Wait for the turn to resolve",
  }}
  template={frame}
/>

<!-- As the game shows it: cancelling and demolishing have no order in the simulation yet. -->
<Story
  name="Without cancel or demolish"
  args={{
    slot: { state: "built", building: building("mine", "Mine", 2) },
    onCancel: undefined,
    onDemolish: undefined,
  }}
  template={frame}
/>
