<script lang="ts">
  import Icon from "../../../../shared/ui/Icon.svelte";
  import {
    RESOURCE_NAMES,
    type Resource,
    type ResourceStock,
    formatChange,
    formatQuantity,
    netIncome,
  } from "../../resources";
  import { type ConceptChain, conceptOwner } from "../concepts/chain.svelte";

  type GameToolbarResourceProps = {
    resource: Resource;
    stock: ResourceStock;
    /** The chain the figure opens its concept's card in. Without one, the figure only shows. */
    concepts?: ConceptChain;
  };
  const { resource, stock, concepts }: GameToolbarResourceProps = $props();

  const net = $derived(netIncome(stock));
  // lit while its card is the chain's first, the pointer being in the card rather than on the figure
  const active = $derived(concepts?.levels[0]?.concept.id === resource);
</script>

<!-- A button for keyboard focus to reach it: focusing a figure opens its card at once. -->
<button type="button" class:active {@attach concepts && conceptOwner(concepts, resource)}>
  <Icon name={resource} size="lg" label={RESOURCE_NAMES[resource]} style="color: var(--color-resource-{resource})" />
  {formatQuantity(stock.quantity)}
  <span class="net" class:surplus={net > 0} class:deficit={net < 0}>
    {formatChange(net)}<span class="sr-only"> per turn</span>
  </span>
</button>

<style lang="postcss">
  @reference "../../../../index.css";

  button {
    @apply flex h-full cursor-default items-center gap-1.5 px-2.5 font-display text-lg font-semibold tabular-nums;
    @apply transition-colors duration-150 outline-none;
    &:hover,
    &:focus-visible,
    &.active {
      @apply bg-surface-raised shadow-[inset_0_-2px_0_var(--color-accent)];
    }
  }

  .net {
    @apply text-sm text-fg-subtle;
  }

  .surplus {
    @apply text-success;
  }
  .deficit {
    @apply text-danger;
  }
</style>
