<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";

  import { GAME_CONCEPTS } from "../../concepts";
  import ConceptCard from "./ConceptCard.svelte";

  const { Story } = defineMeta({
    title: "Game/Concepts/ConceptCard",
    component: ConceptCard,
    tags: ["autodocs"],
    args: { concept: GAME_CONCEPTS.population, meta: "3,420" },
    argTypes: {
      dwell: { control: { type: "range", min: 0, max: 1, step: 0.05 } },
      depth: { control: { type: "range", min: 0, max: 3, step: 1 } },
    },
    parameters: { layout: "centered" },
  });
</script>

{#snippet rows()}
  <div class="my-2 h-px bg-line"></div>
  <div class="flex justify-between gap-4 leading-[22px]">
    <span>Growth</span><span class="font-display text-[15px] font-semibold text-success tabular-nums">+1.2%</span>
  </div>
  <div class="flex justify-between gap-4 leading-[22px]">
    <span>Housing used</span><span class="font-display text-[15px] font-semibold text-fg tabular-nums"
      >2,900 / 3,600</span
    >
  </div>
{/snippet}

<!-- The bar fills while the pointer rests on the card's owner: the links are plain words until it is full. -->
<Story name="Filling" args={{ dwell: 0.4, open: false, children: rows }} />

<Story name="Open" args={{ dwell: 1, open: true, children: rows }} />

<Story name="Opened by keyboard" args={{ dwell: 1, open: true, byKeyboard: true, children: rows }} />

<Story name="Last of the chain" args={{ dwell: 1, open: true, depth: 3, children: rows }} />

<Story name="Kind instead of a figure" args={{ concept: GAME_CONCEPTS["fission-reactor"], meta: undefined }} />
