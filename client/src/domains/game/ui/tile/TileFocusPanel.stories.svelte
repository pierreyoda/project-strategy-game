<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import type { ComponentProps } from "svelte";
  import { fn } from "storybook/test";

  import { slotOf } from "../../state/tileDetails";
  import TileFocusPanel from "./TileFocusPanel.svelte";
  import { FIGURES, HILLS, SPARSE, YOU } from "./tileSamples";

  const { Story } = defineMeta({
    title: "Game/Tile/TileFocusPanel",
    component: TileFocusPanel,
    tags: ["autodocs"],
    args: { viewer: YOU.nation, tint: YOU.color, onCentre: fn(), onClose: fn() },
    // the game's height under its toolbar, at the smallest window it is designed for
    parameters: { layout: "fullscreen" },
  });
</script>

{#snippet docked(args: ComponentProps<typeof TileFocusPanel>)}
  <div class="h-[772px] bg-ground">
    <TileFocusPanel {...args} />
  </div>
{/snippet}

<!-- The board's tile, every row filled in: what the panel is drawn for. -->
<Story
  name="Everything known"
  args={{ details: HILLS, slot: slotOf(HILLS, YOU.nation), figures: FIGURES }}
  template={docked}
/>

<!-- What the game can tell today: the projection's rows only, the actions disabled. -->
<Story
  name="As the game shows it today"
  args={{ details: SPARSE, slot: slotOf(SPARSE, YOU.nation), unavailable: "Not in the simulation yet" }}
  template={docked}
/>
