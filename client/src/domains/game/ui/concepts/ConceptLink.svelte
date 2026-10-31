<script lang="ts">
  import { type ConceptId, lookupGameConcept } from "../../concepts";
  import type { ConceptChain } from "./chain.svelte";

  type ConceptLinkProps = {
    /** The concept the words name. */
    id: ConceptId;
    /** The words themselves, the concept's title unless said otherwise. */
    text?: string;
    /** Depth of the card the link sits in, for the chain to open the next card beneath it. */
    depth: number;
    /** The chain to report to. Without one, the link is only styled: a card shown on its own, as in a story. */
    chain?: ConceptChain;
    /** Plain words: the chain goes no deeper, as in its last card. */
    plain?: boolean;
  };
  const { id, text, depth, chain, plain = false }: ConceptLinkProps = $props();

  const concept = $derived(lookupGameConcept(id));
  const label = $derived(text ?? concept?.title ?? id);

  let node = $state<HTMLButtonElement>();
</script>

<!--
  Not a link to anywhere: a button that opens the next card in place. Accent words with a dotted
  underline, the one typographic signal the whole system rests on.
-->
{#if plain || !concept}
  <span>{label}</span>
{:else}
  <button
    bind:this={node}
    type="button"
    class={[
      "cursor-help rounded-[3px] text-accent underline decoration-accent/55 decoration-dotted decoration-1 underline-offset-[3px]",
      "transition-colors duration-150 hover:bg-accent/20 hover:text-fg hover:decoration-accent hover:decoration-solid hover:ring-[3px] hover:ring-accent/20",
      "focus-visible:decoration-solid focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
    ]}
    aria-haspopup="dialog"
    onpointerenter={() => node && chain?.hoverLink(depth, id, node)}
    onpointerleave={() => chain?.leaveLink(depth)}
    onclick={() => node && chain?.openLink(depth, id, node)}
  >
    {label}
  </button>
{/if}
