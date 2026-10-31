<script lang="ts">
  import { type ResourceStock, formatChange, netIncome } from "../../resources";

  type ResourceBreakdownProps = {
    stock: ResourceStock;
  };
  const { stock }: ResourceBreakdownProps = $props();

  const net = $derived(netIncome(stock));
  /** Turns before the stock runs out at this pace, `0` when it already has. */
  const turnsLeft = $derived(net < 0 ? Math.ceil(stock.quantity / -net) : null);
</script>

<!-- The figures of a resource's card: what the concept is stays in its summary, what is true of it now goes here. -->
<div class="rule"></div>
<!-- colored only when there is something to color: a 0 is neither a gain nor a loss -->
<div class="row">
  <span>Produced</span><span class="num" class:surplus={stock.income > 0}>{formatChange(stock.income)}</span>
</div>
<div class="row">
  <span>Consumed</span><span class="num" class:deficit={stock.depletion > 0}>{formatChange(-stock.depletion)}</span>
</div>
<div class="rule"></div>
<div class="row total">
  <span>Per turn</span>
  <span class="num" class:surplus={net > 0} class:deficit={net < 0}>{formatChange(net)}</span>
</div>
{#if turnsLeft !== null}
  <div class="row warning">
    <span>Runs out</span>
    <span class="num">{turnsLeft === 0 ? "now" : `in ${turnsLeft} turn${turnsLeft === 1 ? "" : "s"}`}</span>
  </div>
{/if}

<style lang="postcss">
  @reference "../../../../index.css";

  .rule {
    @apply my-2 h-px bg-line;
  }

  .row {
    @apply flex justify-between gap-4 leading-[22px];
  }

  .num {
    @apply font-display text-[15px] font-semibold text-fg tabular-nums;
  }

  .total {
    @apply font-display font-semibold tracking-wider text-fg uppercase;
  }

  .warning {
    @apply text-warning;
    .num {
      @apply text-warning;
    }
  }

  .surplus {
    @apply text-success;
  }
  .deficit {
    @apply text-danger;
  }
</style>
