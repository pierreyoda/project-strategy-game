<script lang="ts" module>
  import { onDestroy, onMount } from "svelte";

  import Button from "../../../shared/ui/Button.svelte";
  import type { OffsetCoords } from "../../map/hexMap";
  import { TERRAIN_NAMES, type TerrainKind, isExplored } from "../../map/terrain";
  import { type GameEvent, NO_NATION, TURN_PHASE_NAMES, TileVisibility } from "../session/protocol";
  import { formatQuantity } from "../resources";
  import { type GameLogEntry, createActiveGame } from "../state/activeGame.svelte";
  import { createConceptChain } from "./concepts/chain.svelte";
  import ConceptChain from "./concepts/ConceptChain.svelte";
  import ResourceBreakdown from "./concepts/ResourceBreakdown.svelte";
  import GameView from "./GameView.svelte";
  import { nationColor } from "../nations";
  import TileFocusPanel from "./tile/TileFocusPanel.svelte";
  import TilePanel from "./tile/TilePanel.svelte";
  import GameToolbar from "./toolbar/GameToolbar.svelte";
  import UnitPanel from "./UnitPanel.svelte";

  /** How many lines of the turn log are shown at once. */
  const LOG_SHOWN = 8;
  /** Width of the docked tile focus panel, which the camera frames its tile beside (`w-96`). */
  const FOCUS_PANEL_WIDTH = 384;
  /** Why a tile's building slot cannot be acted on: the simulation has no slots yet. */
  const SLOT_UNAVAILABLE = "Not in the simulation yet";

  const VISIBILITY_NAMES: Record<number, string> = {
    [TileVisibility.Unexplored]: "Unexplored",
    [TileVisibility.Fogged]: "Out of sight",
    [TileVisibility.Visible]: "In sight",
  };

  /**
   * What an event reads like in the turn log, or `null` for the ones not worth a line - or not the
   * viewer's to read.
   */
  const describeEvent = (event: GameEvent, viewer: number): string | null => {
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
      case "controllerLog":
        // a controller's own diagnostics: the viewer's to read when it plays for the viewer's nation
        // (their automation, once there is any - a disloyal general will need hiding again), and
        // otherwise a look into a rival's planning, shown only to whoever is writing the scripts
        if (event.nation !== viewer && !import.meta.env.DEV) {
          return null;
        }
        return `Nation ${event.nation + 1} (${event.controller}): ${event.message}`;
    }
  };

  /** Every word the player reads of the turn log is decided here, not in `activeGame`. */
  const describe = (entry: GameLogEntry, viewer: number): string | null => {
    switch (entry.kind) {
      case "event":
        return describeEvent(entry.event, viewer);
      case "rejected":
        return entry.rejection.message;
      case "turnCancelled":
        return "The turn was interrupted.";
    }
  };
</script>

<script lang="ts">
  const game = createActiveGame();
  /** The concept cards of the whole view: one chain, whatever opened it. */
  const concepts = createConceptChain();

  let hoveredTile = $state<OffsetCoords | null>(null);

  // a primitive, so that the camera frames again when the focus moves, not on every projection
  const focusedIndex = $derived(game.focusOpen ? (game.inspected?.index ?? null) : null);
  /** Bumped by the focus panel's centre button, for the camera to frame its tile again. */
  let recentre = $state(0);
  const framing = $derived.by(() => {
    void recentre;
    return focusedIndex === null ? null : { tile: focusedIndex, minZoom: 2, insetLeft: FOCUS_PANEL_WIDTH };
  });

  /**
   * Escape closes the deepest thing open: a concept card first, which the chain does on its own -
   * read here before it does, in the capture phase - then the focus view, the tile panel, the unit.
   */
  const onKeydown = (event: KeyboardEvent) => {
    if (event.key === "Escape" && concepts.levels.length === 0) {
      game.escape();
    }
  };

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
      .map((entry) => describe(entry, game.viewer))
      // filtered before it is cut, so that the lines nobody reads do not crowd out the others
      .filter((line) => line !== null)
      .slice(-LOG_SHOWN),
  );

  onMount(() => void game.startNewGame());
  onDestroy(() => game.dispose());
</script>

<svelte:window onkeydowncapture={onKeydown} />

<div class="flex h-screen flex-col">
  {#if game.world}
    <GameToolbar turn={game.turn} resources={game.stocks} {concepts} />
  {/if}

  <div class="relative min-h-0 flex-1">
    {#if game.map && game.world}
      <GameView
        map={game.map}
        world={game.world}
        dirtyTiles={game.dirtyTiles}
        entities={game.entities}
        deposits={game.deposits}
        focusTile={game.focusTile}
        selectedTile={game.selected?.tile ?? null}
        reach={game.reach}
        previewPath={game.previewPath}
        viewer={game.viewer}
        onTileHover={(tile) => {
          hoveredTile = tile;
          game.hoverTile(tile);
        }}
        onTileClick={(tile) => void game.selectOrInspect(tile)}
        onTileOrder={(tile) => void game.orderMove(tile)}
        {framing}
        focusedTile={focusedIndex}
      />
    {:else}
      <p class="grid size-full place-items-center text-fg-muted">
        {game.starting ? "Generating the planet…" : "No game."}
      </p>
    {/if}

    {#if game.inspected && game.inspectedSlot && game.focusOpen}
      <div class="focus-panel">
        <TileFocusPanel
          details={game.inspected}
          slot={game.inspectedSlot}
          viewer={game.viewer}
          tint={nationColor(game.viewer)}
          chain={concepts}
          unavailable={SLOT_UNAVAILABLE}
          onCentre={() => recentre++}
          onClose={() => game.closeFocus()}
        />
      </div>
    {:else if game.inspected}
      <div class="tile-panel">
        <TilePanel
          details={game.inspected}
          viewer={game.viewer}
          chain={concepts}
          onDetails={() => game.openFocus()}
          onClose={() => game.closeInspection()}
        />
      </div>
    {/if}

    {#if game.selected}
      <div class="unit-panel">
        <UnitPanel
          selected={game.selected}
          viewer={game.viewer}
          hasNext={game.hasNextUnit}
          onSkip={() => game.skipUnit()}
          onNext={() => game.nextUnit()}
        />
      </div>
    {/if}

    {#if lines.length > 0}
      <ul class="log" class:beside-focus={game.focusOpen} aria-label="Turn log">
        {#each lines as line, index (`${index}-${line}`)}
          <li>{line}</li>
        {/each}
      </ul>
    {/if}

    <div class="controls">
      {#if game.error}
        <span class="text-danger">{game.error}</span>
      {:else if game.selected}
        <span class="text-fg-muted">Right-click a highlighted tile to move the unit · Esc to deselect</span>
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

<!-- The viewer's own figures, which the concepts cannot know: a resource's card shows its stock as of this turn. -->
<ConceptChain
  chain={concepts}
  meta={(concept) => concept.resource && `${formatQuantity(game.stocks[concept.resource].quantity)} stored`}
>
  {#snippet extra(concept)}
    {#if concept.resource}
      <ResourceBreakdown stock={game.stocks[concept.resource]} />
    {/if}
  {/snippet}
</ConceptChain>

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

  .unit-panel {
    @apply absolute right-3 bottom-14;
  }
  .tile-panel {
    @apply absolute top-3 left-3;
  }

  .focus-panel {
    @apply absolute inset-y-0 left-0 z-10;
  }

  .log.beside-focus {
    @apply left-99;
  }
</style>
