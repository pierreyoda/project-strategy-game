<script lang="ts">
  import { type ConceptId, lookupGameConcept } from "../../concepts";
  import { type ConceptChain, conceptOwner } from "./chain.svelte";

  type ConceptTermProps = {
    id: ConceptId;
    /** The words shown, the concept's title unless said otherwise. */
    text?: string;
    /** The chain the term opens its card in. Without one, the term is only styled, as in a story. */
    chain?: ConceptChain;
  };
  const { id, text, chain }: ConceptTermProps = $props();

  const label = $derived(text ?? lookupGameConcept(id)?.title ?? id);
</script>

<!--
  A game concept named in a panel rather than in a card: the panel's words are a card's owner, as a
  toolbar figure is. Same dotted accent as the links inside cards.
-->
<button
  type="button"
  class={[
    "cursor-help rounded-[3px] text-accent underline decoration-accent/55 decoration-dotted decoration-1 underline-offset-[3px]",
    "transition-colors duration-150 hover:bg-accent/20 hover:text-fg hover:decoration-accent hover:decoration-solid",
    "focus-visible:decoration-solid focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
  ]}
  {@attach chain && conceptOwner(chain, id)}
>
  {label}
</button>
