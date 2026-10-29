<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";

  import type { Resource, ResourceStock } from "../../resources";
  import GameToolbar from "./GameToolbar.svelte";

  const { Story } = defineMeta({
    title: "Game/Toolbar",
    component: GameToolbar,
    tags: ["autodocs"],
    parameters: {
      layout: "fullscreen",
    },
  });

  const midGame: Record<Resource, ResourceStock> = {
    credits: { quantity: 1240, income: 52, depletion: 14 },
    water: { quantity: 860, income: 20, depletion: 8 },
    food: { quantity: 512, income: 30, depletion: 36 },
    electricity: { quantity: 128, income: 18, depletion: 4 },
    metals: { quantity: 96, income: 3, depletion: 5 },
    oil: { quantity: 54, income: 5, depletion: 0 },
    uranium: { quantity: 7, income: 1, depletion: 0 },
  };

  // quantities are `u16` in the `core` crate
  const extremes: Record<Resource, ResourceStock> = {
    credits: { quantity: 65535, income: 0, depletion: 0 },
    water: { quantity: 0, income: 0, depletion: 0 },
    food: { quantity: 0, income: 0, depletion: 1200 },
    electricity: { quantity: 30000, income: 4500, depletion: 0 },
    metals: { quantity: 65535, income: 120, depletion: 120 },
    oil: { quantity: 9, income: 0, depletion: 0 },
    uranium: { quantity: 0, income: 0, depletion: 0 },
  };
</script>

<Story name="Mid game" args={{ turn: 42, resources: midGame }} />

<Story name="Extremes" args={{ turn: 1337, resources: extremes }} />
