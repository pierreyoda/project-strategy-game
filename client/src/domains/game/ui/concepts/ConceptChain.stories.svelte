<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";

  import Icon from "../../../../shared/ui/Icon.svelte";
  import { type GameConcept } from "../../concepts";
  import { conceptOwner, createConceptChain } from "./chain.svelte";
  import ConceptChain from "./ConceptChain.svelte";

  // no `component` here: the story renders its own markup, which svelte-csf would otherwise nest inside it
  const { Story } = defineMeta({
    title: "Game/Concepts/ConceptChain",
    parameters: {
      layout: "fullscreen",
      docs: { story: { height: "560px", inline: false } },
    },
  });

  /** The figures of the moment, which the concepts themselves cannot know: here, made up. */
  const meta = (concept: GameConcept) =>
    ({ electricity: "128 stored", uranium: "7 in stock", population: "3,420" })[concept.id];
</script>

<script lang="ts">
  const chain = createConceptChain();
</script>

<!--
  Rest the pointer on a figure: its card appears, its bar fills, then the card may be entered and its
  underlined words lead on - electricity to a fission reactor to uranium to a deposit. Tab to a figure
  for the keyboard route; Escape closes the deepest card.
-->
<Story name="From the HUD">
  <div class="h-[560px] bg-ground">
    <div class="flex h-12 items-center gap-2 border-b border-line bg-surface px-3">
      {#each [["electricity", "128", "+14", "text-success"], ["uranium", "7", "−1", "text-danger"]] as const as [id, stock, delta, tone] (id)}
        <button
          type="button"
          class="flex h-full items-center gap-1.5 px-2.5 font-display text-lg font-semibold tabular-nums hover:bg-surface-raised hover:shadow-[inset_0_-2px_0_var(--color-accent)]"
          {@attach conceptOwner(chain, id)}
        >
          <Icon name={id} size="lg" class="text-resource-{id}" />
          {stock}
          <span class="text-[13px] {tone}">{delta}</span>
        </button>
      {/each}
    </div>
    <p class="m-6 max-w-md text-sm text-fg-muted">
      Hover a figure and wait for the bar. Once it pulses, move into the card and hover a concept.
    </p>
  </div>
  <ConceptChain {chain} meta={(concept) => meta(concept as GameConcept)} />
</Story>
