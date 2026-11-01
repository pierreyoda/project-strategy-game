<script lang="ts">
  import Badge from "../../../../shared/ui/Badge.svelte";
  import Button from "../../../../shared/ui/Button.svelte";
  import Icon from "../../../../shared/ui/Icon.svelte";
  import EntityPortrait from "../../../map/components/EntityPortrait.svelte";
  import { buildingDrawing } from "../../../map/entities";
  import { TERRAIN_COLORS } from "../../../map/terrain";
  import { RESOURCE_NAMES, formatChange, formatQuantity } from "../../resources";
  import { TileVisibility } from "../../session/protocol";
  import type { SlotView, TileDetails } from "../../state/tileDetails";
  import type { ConceptChain } from "../concepts/chain.svelte";
  import ConceptTerm from "../concepts/ConceptTerm.svelte";
  import BuildingSlot, { type SlotFigures } from "./BuildingSlot.svelte";
  import { VISIBILITY_NAMES, cssColor, ownershipColor, ownershipText } from "./tileText";

  type TileFocusPanelProps = {
    details: TileDetails;
    slot: SlotView;
    /** The nation being played: "yours", and the colour of its buildings' marks. */
    viewer: number;
    /** The colour of the viewer's nation, for the art of a building of theirs. */
    tint: number;
    chain?: ConceptChain;
    /** Why the slot's actions cannot be taken; `null` lets them be. */
    unavailable?: string | null;
    figures?: SlotFigures;
    onCentre: () => void;
    onClose: () => void;
  };
  const {
    details,
    slot,
    viewer,
    tint,
    chain,
    unavailable = null,
    figures,
    onCentre,
    onClose,
  }: TileFocusPanelProps = $props();

  const ownership = $derived(ownershipText(details, viewer));
  const portrait = $derived(details.building && buildingDrawing(details.building.type, details.building.level));
  const portraitTint = $derived(details.owner ? details.owner.color : tint);
  const hasNetwork = $derived(details.supplyNode || details.infrastructure);
  const HEX = "50,3 97,30 97,85.5 50,112.5 3,85.5 3,30";
</script>

<!--
  Everything known of one tile, docked at full height: what Details on the tile panel opens. The
  camera frames the tile in the room left beside it (see `GameView`'s `framing`).
-->
<section
  class="flex h-full w-96 flex-col border-r border-line bg-surface/95 backdrop-blur-md"
  aria-labelledby="tile-focus-title"
>
  <header class="flex min-h-9 items-center gap-1 border-b border-line py-1 pr-1.5 pl-3">
    <h2 id="tile-focus-title" class="font-display text-sm font-semibold tracking-wider text-fg-muted uppercase">
      Tile {details.coords.col}, {details.coords.row}
    </h2>
    <Button
      variant="ghost"
      size="sm"
      icon="locate"
      class="ml-auto"
      aria-label="Centre the camera on the tile"
      onclick={onCentre}
    />
    <Button variant="ghost" size="sm" icon="close" aria-label="Close" onclick={onClose} />
  </header>

  <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-3">
    <div class="flex items-center gap-3.5">
      <div class="relative h-22 w-19 shrink-0">
        <svg class="absolute inset-0 size-full" viewBox="0 0 100 115.5" preserveAspectRatio="none" aria-hidden="true">
          <polygon
            points={HEX}
            fill={details.terrain === null ? "var(--color-surface-sunken)" : cssColor(TERRAIN_COLORS[details.terrain])}
            stroke="var(--color-line-strong)"
            stroke-width="1.5"
          />
        </svg>
        {#if portrait}
          <EntityPortrait
            sprite={portrait.sprite}
            tint={portraitTint}
            width={80}
            height={92}
            class="absolute -top-0.5 -left-0.5"
          />
        {/if}
      </div>
      <div class="flex min-w-0 flex-col gap-1.5">
        <div class="font-display text-2xl leading-[26px] font-bold tracking-wide uppercase">{details.terrainName}</div>
        <div class="flex flex-wrap gap-1.5">
          <Badge
            size="sm"
            tone={details.visibility === TileVisibility.Visible ? "accent" : "neutral"}
            icon="visibility"
          >
            {VISIBILITY_NAMES[details.visibility]}
          </Badge>
          {#if details.visibility === TileVisibility.Fogged}
            <ConceptTerm id="fog-of-war" text="as last seen" {chain} />
          {/if}
        </div>
        {#if ownership}
          <div class="flex items-center gap-1.5 text-[13px] text-fg-muted">
            <i class="inline-block size-2.5 rounded-[2px]" style:background={ownershipColor(details)}></i>{ownership}
          </div>
        {/if}
      </div>
    </div>

    <BuildingSlot {slot} tint={portraitTint} {chain} {unavailable} {figures} />

    {#if details.terrain !== null}
      <div class="flex flex-col gap-1.5">
        <div class="group-label">Ground</div>
        <dl class="rows">
          <dt><Icon name="elevation" size="sm" />Elevation</dt>
          <dd>{formatQuantity(details.elevation)} m</dd>
          {#if details.movementCost !== undefined}
            <dt><Icon name="movement" size="sm" />Movement cost</dt>
            <dd>{details.movementCost}</dd>
          {/if}
          {#if details.defense !== undefined}
            <dt><Icon name="defense" size="sm" />Defense</dt>
            <dd class={details.defense > 0 ? "text-success" : details.defense < 0 ? "text-danger" : ""}>
              {formatChange(Math.round(details.defense * 100))}%
            </dd>
          {/if}
        </dl>
      </div>
    {/if}

    {#if details.deposit || details.hasDeposit || details.yields}
      <div class="flex flex-col gap-1.5">
        <div class="group-label">Resources</div>
        {#if details.deposit}
          <div
            class="flex items-start gap-2.5 rounded-tl-cut-md rounded-br-cut-md border border-line bg-surface-sunken p-2 chamfer"
          >
            <Icon
              name={details.deposit.resource}
              size="lg"
              style="color: var(--color-resource-{details.deposit.resource})"
            />
            <div class="min-w-0 flex-1">
              <div class="flex items-center justify-between gap-2 font-medium">
                <span>{RESOURCE_NAMES[details.deposit.resource]} <ConceptTerm id="deposit" {chain} /></span>
                {#if details.deposit.turnsLeft !== null}<Badge tone="warning" size="sm">Depleting</Badge>{/if}
              </div>
              <div class="mt-0.5 text-xs text-fg-subtle">
                {#if details.deposit.recharge}
                  {formatChange(details.deposit.recharge)} per turn, springs back
                {:else}
                  {formatQuantity(details.deposit.left)} left{details.deposit.turnsLeft !== null
                    ? ` · runs out in ${details.deposit.turnsLeft} turns`
                    : ""}
                {/if}
              </div>
            </div>
          </div>
        {:else if details.hasDeposit}
          <dl class="rows">
            <dt><Icon name="metals" size="sm" /><ConceptTerm id="deposit" {chain} /></dt>
            <dd>Surveyed</dd>
          </dl>
        {/if}
        {#if details.yields}
          <dl class="rows">
            {#each Object.entries(details.yields) as [resource, amount] (resource)}
              <dt>
                <Icon
                  name={resource as keyof typeof RESOURCE_NAMES}
                  size="sm"
                  style="color: var(--color-resource-{resource})"
                />{RESOURCE_NAMES[resource as keyof typeof RESOURCE_NAMES]}
              </dt>
              <dd class="text-success">{formatChange(amount ?? 0)} / turn</dd>
            {/each}
          </dl>
        {/if}
      </div>
    {/if}

    {#if hasNetwork}
      <div class="flex flex-col gap-1.5">
        <div class="group-label">Network</div>
        <dl class="rows">
          {#if details.supplyNode}
            <dt><Icon name="supply" size="sm" /><ConceptTerm id="supply" text="Supply node" {chain} /></dt>
            <dd>On this tile</dd>
          {/if}
          {#if details.infrastructure}
            <dt><Icon name="supply" size="sm" />Infrastructure</dt>
            <dd>Built</dd>
          {/if}
        </dl>
      </div>
    {/if}
  </div>

  <footer class="flex items-center gap-1.5 border-t border-line px-3 py-2 text-xs text-fg-subtle">
    <kbd class="kbd">Esc</kbd> close · click another tile to move the focus
  </footer>
</section>

<style>
  .group-label {
    font: 600 11px/16px var(--font-display);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-fg-subtle);
  }
  .rows {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 6px 12px;
    margin: 0;
    font-size: 14px;
  }
  .rows :global(dt) {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--color-fg-muted);
  }
  .rows :global(dd) {
    margin: 0;
    text-align: right;
    font: 600 15px/20px var(--font-display);
    font-variant-numeric: tabular-nums;
  }
  .kbd {
    padding: 0 6px;
    border: 1px solid var(--color-line-strong);
    border-radius: 4px;
    corner-shape: bevel;
    background: var(--color-surface-raised);
    font: 600 11px/16px var(--font-display);
    color: var(--color-fg-muted);
  }
</style>
