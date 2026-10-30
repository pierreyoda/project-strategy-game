<script lang="ts">
  import GameSessionView from "./domains/game/ui/GameSessionView.svelte";
  import PlanetGeneratorTestBench from "./domains/map/components/PlanetGeneratorTestBench.svelte";
  import Button from "./shared/ui/Button.svelte";

  /** Until there is a main menu, the two things there are to look at. */
  let view = $state<"game" | "planets">("game");
</script>

<div class="relative size-full">
  {#if view === "game"}
    <GameSessionView />
  {:else}
    <PlanetGeneratorTestBench />
  {/if}

  <div class="absolute bottom-3 left-3">
    <Button size="sm" variant="ghost" onclick={() => (view = view === "game" ? "planets" : "game")}>
      {view === "game" ? "Planet generator" : "Game"}
    </Button>
  </div>
</div>
