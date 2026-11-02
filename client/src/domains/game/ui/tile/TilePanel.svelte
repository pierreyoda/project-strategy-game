<script lang="ts">
  import Badge from "../../../../shared/ui/Badge.svelte";
  import Button from "../../../../shared/ui/Button.svelte";
  import Icon from "../../../../shared/ui/Icon.svelte";
  import Panel from "../../../../shared/ui/Panel.svelte";
  import { RESOURCE_NAMES } from "../../resources";
  import { TileVisibility } from "../../session/protocol";
  import type { TileDetails } from "../../state/tileDetails";
  import type { ConceptChain } from "../concepts/chain.svelte";
  import ConceptTerm from "../concepts/ConceptTerm.svelte";
  import { VISIBILITY_NAMES, ownershipColor, ownershipText } from "./tileText";

  type TilePanelProps = {
    details: TileDetails;
    /** The nation being played, for the panel to say "yours". */
    viewer: number;
    chain?: ConceptChain;
    onDetails: () => void;
    onClose: () => void;
  };
  const { details, viewer, chain, onDetails, onClose }: TilePanelProps = $props();

  const ownership = $derived(ownershipText(details, viewer));
  const explored = $derived(details.terrain !== null);
</script>

<!-- What a click on a tile shows: the gist, and the way to all of it. -->
<Panel title="Tile {details.coords.col}, {details.coords.row}" variant="glass" padding="sm" class="w-78">
  {#snippet actions()}
    <Button variant="ghost" size="sm" icon="close" aria-label="Close" onclick={onClose} />
  {/snippet}

  <div class="flex flex-col gap-3">
    <div>
      <div class="text-base font-medium">{details.terrainName}</div>
      <div class="mt-1.5 flex flex-wrap gap-1.5">
        <Badge size="sm" tone={details.visibility === TileVisibility.Visible ? "accent" : "neutral"}>
          {VISIBILITY_NAMES[details.visibility]}
        </Badge>
        {#if details.settlement}
          <Badge size="sm">{details.settlement}</Badge>
        {/if}
        {#if details.building}
          <Badge size="sm"
            >{details.building.name} · {details.building.level === 0
              ? "being built"
              : `lvl ${details.building.level}`}</Badge
          >
        {/if}
      </div>
    </div>

    {#if explored}
      <dl class="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1.5 text-sm">
        <dt class="flex items-center gap-2 text-fg-muted"><Icon name="elevation" size="sm" />Elevation</dt>
        <dd class="text-right font-display font-semibold tabular-nums">{details.elevation} m</dd>
        {#if details.deposit}
          <dt class="flex items-center gap-2 text-fg-muted">
            <Icon
              name={details.deposit.resource}
              size="sm"
              style="color: var(--color-resource-{details.deposit.resource})"
            /><ConceptTerm id="deposit" {chain} />
          </dt>
          <dd class="text-right font-display font-semibold">{RESOURCE_NAMES[details.deposit.resource]}</dd>
        {:else if details.hasDeposit}
          <dt class="flex items-center gap-2 text-fg-muted">
            <Icon name="metals" size="sm" /><ConceptTerm id="deposit" {chain} />
          </dt>
          <dd class="text-right font-display font-semibold">Surveyed</dd>
        {/if}
      </dl>
    {/if}

    {#if ownership}
      <div class="flex items-center gap-1.5 text-[13px] text-fg-muted">
        <i class="inline-block size-2.5 rounded-[2px]" style:background={ownershipColor(details)}></i>{ownership}
      </div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="primary" size="sm" disabled={!explored} onclick={onDetails}>Details</Button>
  {/snippet}
</Panel>
