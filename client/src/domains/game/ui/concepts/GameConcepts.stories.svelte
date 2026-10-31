<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";

  import { GAME_CONCEPTS } from "../../concepts";
  import ConceptCard from "./ConceptCard.svelte";

  const { Story } = defineMeta({
    title: "Game/Concepts/All concepts",
    parameters: { layout: "fullscreen" },
  });

  const concepts = Object.values(GAME_CONCEPTS);
</script>

<!-- Every concept the game can explain, as its card opens: the registry, read in one glance. -->
<Story name="Registry">
  <div class="grid grid-cols-[repeat(auto-fill,20rem)] gap-6 bg-ground p-6">
    {#each concepts as concept (concept.id)}
      <ConceptCard {concept} />
    {/each}
  </div>
</Story>
