<script lang="ts" module>
  import { onDestroy, onMount } from "svelte";

  import Button from "../../../shared/ui/Button.svelte";
  import type { OffsetCoords } from "../../map/hexMap";
  import { TERRAIN_NAMES, type TerrainKind, isExplored } from "../../map/terrain";
  import { type GameEvent, NO_NATION, TURN_PHASE_NAMES, TileVisibility } from "../session/protocol";
  import { type GameLogEntry, createActiveGame } from "../state/activeGame.svelte";
  import GameView from "./GameView.svelte";
  import GameToolbar from "./toolbar/GameToolbar.svelte";

  /** How many lines of the turn log are shown at once. */
  const LOG_SHOWN = 8;

  const VISIBILITY_NAMES: Record<number, string> = {
    [TileVisibility.Unexplored]: "Unexplored",
    [TileVisibility.Fogged]: "Out of sight",
    [TileVisibility.Visible]: "In sight",
  };

  /** What an event reads like in the turn log, or `null` for the ones not worth a line. */
  const describeEvent = (event: GameEvent): string | null => {
    switch (event.type) {
      case "turnStarted":
        return `— Turn ${event.turn} —`;
      case "log":
        return event.message;
      case "settlementFounded":
        return `Nation ${event.nation + 1} founded a settlement.`;
      case "buildingConstructed":
        return `A building was completed.`;
      case "resourceShortage":
        return `Nation ${event.nation + 1} has run out of ${event.resource}.`;
      case "commandRejected":
        // an AI order the simulation refused: a bug report, not gameplay
        return `Nation ${event.nation + 1} was refused an order: ${event.reason}`;
      case "unitMoved":
        // every step of every unit would drown everything else
        return null;
    }
  };

  /** Every word the player reads of the turn log is decided here, not in `activeGame`. */
  const describe = (entry: GameLogEntry): string | null => {
    switch (entry.kind) {
      case "event":
        return describeEvent(entry.event);
      case "rejected":
        return entry.rejection.message;
      case "turnCancelled":
        return "The turn was interrupted.";
    }
  };
</script>

<script lang="ts">
  const game = createActiveGame();

  let hoveredTile = $state<OffsetCoords | null>(null);

  const hovered = $derived.by(() => {
    const world = game.world;
    if (!world || !hoveredTile) {
      return null;
    }
    const index = hoveredTile.row * world.columns + hoveredTile.col;
    const terrain = world.terrain[index];
    return {
      coords: hoveredTile,
      visibility: VISIBILITY_NAMES[world.visibility[index]] ?? "",
      // the terrain of a tile the viewer has never explored never left the simulation
      terrain: isExplored(terrain) ? TERRAIN_NAMES[terrain as TerrainKind] : "Unknown",
      elevation: world.elevations[index],
      owner: world.tileOwner[index],
      sphere: world.tileSphere[index],
    };
  });

  const lines = $derived(
    game.log
      .slice(-LOG_SHOWN)
      .map(describe)
      .filter((line) => line !== null),
  );

  onMount(() => void game.startNewGame());
  onDestroy(() => game.dispose());
</script>

<div class="flex h-screen flex-col">
  {#if game.world}
    <GameToolbar turn={game.turn} resources={game.stocks} />
  {/if}

  <div class="relative min-h-0 flex-1">
    {#if game.map && game.world}
      <GameView
        map={game.map}
        world={game.world}
        dirtyTiles={game.dirtyTiles}
        entities={game.entities}
        focusTile={game.focusTile}
        selectedTile={game.selected?.tile ?? null}
        onTileHover={(tile) => (hoveredTile = tile)}
        onTileClick={(tile) => void game.clickTile(tile)}
      />
    {:else}
      <p class="grid size-full place-items-center text-fg-muted">
        {game.starting ? "Generating the planet…" : "No game."}
      </p>
    {/if}

    {#if lines.length > 0}
      <ul class="log" aria-label="Turn log">
        {#each lines as line, index (`${index}-${line}`)}
          <li>{line}</li>
        {/each}
      </ul>
    {/if}

    <div class="controls">
      {#if game.error}
        <span class="text-danger">{game.error}</span>
      {:else if game.selected}
        <span class="text-fg-muted">Unit {game.selected.id} selected · click a neighbouring tile to move it</span>
      {:else if hovered}
        <span class="text-fg-muted tabular-nums">
          ({hovered.coords.col}, {hovered.coords.row}) · {hovered.terrain} · {hovered.elevation} m ·
          {hovered.visibility}
          {#if hovered.owner !== NO_NATION}
            · Nation {hovered.owner + 1}
          {:else if hovered.sphere !== NO_NATION}
            · sphere of Nation {hovered.sphere + 1}
          {/if}
        </span>
      {/if}
      {#if game.resolving}
        <span class="text-fg-muted">
          {game.phase ? TURN_PHASE_NAMES[game.phase] : "Resolving"} · {game.percent}%
        </span>
        <Button size="sm" onclick={() => game.cancelTurn()}>Interrupt</Button>
      {:else}
        <Button variant="primary" size="sm" icon="turn" disabled={!game.world} onclick={() => void game.endTurn()}>
          End turn
        </Button>
      {/if}
      <Button size="sm" disabled={game.starting || game.resolving} onclick={() => void game.startNewGame()}>
        New game
      </Button>
    </div>
  </div>
</div>

<style lang="postcss">
  @reference "../../../index.css";

  .log {
    @apply absolute bottom-14 left-3 flex max-w-96 flex-col gap-0.5 text-sm text-fg-muted;
    li {
      @apply bg-surface/70 px-2 py-0.5 backdrop-blur-sm;
    }
  }

  .controls {
    @apply absolute right-3 bottom-3 left-3 flex items-center justify-end gap-3 text-sm;
  }
</style>
