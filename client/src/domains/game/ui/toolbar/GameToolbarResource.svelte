<script lang="ts" module>
  const quantityFormat = new Intl.NumberFormat("en");
  const netFormat = new Intl.NumberFormat("en", { signDisplay: "exceptZero" });
</script>

<script lang="ts">
  import Icon from "../../../../shared/ui/Icon.svelte";
  import { RESOURCE_NAMES, type Resource, type ResourceStock, netIncome } from "../../resources";

  type GameToolbarResourceProps = {
    resource: Resource;
    stock: ResourceStock;
  };
  const { resource, stock }: GameToolbarResourceProps = $props();

  const net = $derived(netIncome(stock));
</script>

<div>
  <Icon name={resource} size="lg" label={RESOURCE_NAMES[resource]} style="color: var(--color-resource-{resource})" />
  {quantityFormat.format(stock.quantity)}
  <span class="net" class:surplus={net > 0} class:deficit={net < 0}>
    <!-- a minus sign rather than a hyphen, as wide as the plus sign -->
    {netFormat.format(net).replace("-", "−")}<span class="sr-only"> per turn</span>
  </span>
</div>

<style lang="postcss">
  @reference "../../../../index.css";

  div {
    @apply flex h-full items-center gap-1.5 px-2.5 font-display text-lg font-semibold tabular-nums;
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
