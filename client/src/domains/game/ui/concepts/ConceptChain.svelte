<script lang="ts">
  import type { Snippet } from "svelte";

  import type { GameConcept } from "../../concepts";
  import type { ConceptChain } from "./chain.svelte";
  import ConceptCard from "./ConceptCard.svelte";

  type ConceptChainProps = {
    chain: ConceptChain;
    /** More of a card, under its summary: the figures of the moment, which the concepts themselves cannot know. */
    extra?: Snippet<[GameConcept, number]>;
    /** A figure beside a card's title, for the same reason. */
    meta?: (concept: GameConcept) => string | undefined;
  };
  const { chain, extra, meta }: ConceptChainProps = $props();

  // the cards are gone with the chain's view, whatever the pointer was doing
  $effect(() => () => chain.closeAll());
</script>

<svelte:window onkeydown={(event) => event.key === "Escape" && chain.back()} />

<!-- The first card hangs under its owner, as a tooltip does; the next ones open beside the words that led to them. -->
{#each chain.levels as level, depth (depth)}
  <ConceptCard
    concept={level.concept}
    {depth}
    dwell={level.dwell}
    open={level.open}
    byKeyboard={level.byKeyboard}
    anchor={level.anchor}
    placement={depth === 0 ? "bottom-start" : "right-start"}
    meta={meta?.(level.concept)}
    {chain}
  >
    {#if extra}
      {@render extra(level.concept, depth)}
    {/if}
  </ConceptCard>
{/each}
