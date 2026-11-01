<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import type { ComponentProps } from "svelte";

  import BuildingSlot from "./BuildingSlot.svelte";
  import { FIGURES, RIVAL, YOU } from "./tileSamples";

  const { Story } = defineMeta({
    title: "Game/Tile/BuildingSlot",
    component: BuildingSlot,
    tags: ["autodocs"],
    args: { tint: YOU.color, figures: FIGURES },
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
    slot: { state: "construction", building: { type: "mine", name: "Mine", level: 1 }, turnsLeft: 3, turnsTotal: 5 },
  }}
  template={frame}
/>

<Story
  name="Built"
  args={{ slot: { state: "built", building: { type: "mine", name: "Mine", level: 2 } } }}
  template={frame}
/>

<Story
  name="Built, highest level"
  args={{ slot: { state: "built", building: { type: "solar_plant", name: "Solar power plant", level: 3 } } }}
  template={frame}
/>

<Story
  name="Another nation's"
  args={{
    slot: { state: "foreign", owner: RIVAL, building: { type: "solar_plant", name: "Solar power plant", level: 1 } },
  }}
  template={frame}
/>

<Story name="No slot" args={{ slot: { state: "none", reason: "Nothing can be built on water." } }} template={frame} />

<Story name="Settlement" args={{ slot: { state: "settlement", settlement: "Town" } }} template={frame} />

<!-- As the game shows it today: the simulation cannot build on a tile, so every action says why. -->
<Story
  name="Actions unavailable"
  args={{
    slot: { state: "built", building: { type: "mine", name: "Mine", level: 2 } },
    figures: {},
    unavailable: "Not in the simulation yet",
  }}
  template={frame}
/>
