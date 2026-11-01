<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";

  import { type Lens, NO_LENS, lensFiltered, lensToggled } from "../state/lens";
  import LensPanel from "./LensPanel.svelte";

  const { Story } = defineMeta({
    title: "Game/Lens panel",
    component: LensPanel,
    tags: ["autodocs"],
  });

  const NATIONS = [
    { id: 0, name: "Free Cities of Aurel" },
    { id: 2, name: "Kessrin Directorate" },
    { id: 1, name: "The Southern Compact" },
  ];

  const STARTS = {
    none: NO_LENS,
    resources: { kind: "resources", filter: "all" },
    filtered: { kind: "resources", filter: "oil" },
    political: { kind: "political" },
  } satisfies Record<string, Lens>;
</script>

<script lang="ts">
  // each story starts from its own lens, and plays from there
  const lenses = $state<Record<keyof typeof STARTS, Lens>>({ ...STARTS });
</script>

{#snippet playable(story: keyof typeof STARTS)}
  <!-- stands in for the map, which the panel's glass lets show through -->
  <div
    class="flex justify-end bg-[repeating-linear-gradient(60deg,#173a2a_0_24px,#1d4a8a_24px_48px,#7a5a2c_48px_72px)] p-8"
  >
    <LensPanel
      lens={lenses[story]}
      nations={NATIONS}
      viewer={0}
      onToggle={(kind) => (lenses[story] = lensToggled(lenses[story], kind))}
      onFilter={(filter) => (lenses[story] = lensFiltered(lenses[story], filter))}
    />
  </div>
{/snippet}

<Story name="No lens" asChild>
  {@render playable("none")}
</Story>

<Story name="Resources" asChild>
  {@render playable("resources")}
</Story>

<Story name="Resources, filtered" asChild>
  {@render playable("filtered")}
</Story>

<Story name="Political" asChild>
  {@render playable("political")}
</Story>
