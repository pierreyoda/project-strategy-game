<script lang="ts">
  import { RESOURCES, type Resource, type ResourceStock } from "../../resources";
  import GameToolbarResource from "./GameToolbarResource.svelte";

  type GameToolbarProps = {
    turn: number;
    resources: Record<Resource, ResourceStock>;
  };
  const { turn, resources }: GameToolbarProps = $props();
</script>

<header>
  <ul aria-label="Resources">
    {#each RESOURCES as resource (resource)}
      <li>
        <GameToolbarResource {resource} stock={resources[resource]} />
      </li>
    {/each}
  </ul>
  <div class="turn">
    Turn
    <span>{turn}</span>
  </div>
</header>

<style lang="postcss">
  /* the project's stylesheet rather than "tailwindcss", for `@apply` to know its design tokens */
  @reference "../../../../index.css";

  header {
    @apply flex h-12 w-full items-center justify-between border-b border-line bg-surface/90 px-2 backdrop-blur-md;
  }

  ul {
    @apply flex h-full;
  }

  .turn {
    @apply flex items-baseline gap-1.5 px-2.5 font-display text-2xs font-semibold tracking-widest text-fg-subtle uppercase;
    span {
      @apply text-xl font-bold tracking-normal text-fg tabular-nums;
    }
  }
</style>
