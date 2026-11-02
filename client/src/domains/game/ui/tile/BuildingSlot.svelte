<script lang="ts" module>
  import Badge from "../../../../shared/ui/Badge.svelte";
  import Button from "../../../../shared/ui/Button.svelte";
  import Icon from "../../../../shared/ui/Icon.svelte";
  import type { IconName } from "../../../../shared/ui/icons";
  import Tooltip from "../../../../shared/ui/Tooltip.svelte";
  import EntityPortrait from "../../../map/components/EntityPortrait.svelte";
  import { constructionProgress } from "../../../map/constructions";
  import { buildingDrawing } from "../../../map/entities";
  import { RESOURCES, formatChange, formatQuantity } from "../../resources";
  import { BuildingState, type ResourceAmounts } from "../../session/protocol";
  import { type SlotFigures, type SlotView, buildingName } from "../../state/tileDetails";
  import type { ConceptChain } from "../concepts/chain.svelte";
  import ConceptTerm from "../concepts/ConceptTerm.svelte";

  /** The socket's hexagon, pointy-top in a 100 × 115.5 box, clockwise from its top corner. */
  const HEX = "50,3 97,30 97,85.5 50,112.5 3,85.5 3,30";
  const SOCKET = { width: 96, height: 111 };
  /** The art overflows the socket a little, as a building overhangs its tile on the map. */
  const ART = { width: 108, height: 125 };

  /** Why an action its caller gave no handler for cannot be taken: the simulation has no such order. */
  const NOT_YET = "Not in the simulation yet";

  /** The resources a set of figures names, in the toolbar's order. */
  const named = (amounts: ResourceAmounts | undefined) =>
    RESOURCES.filter((resource) => (amounts?.[resource] ?? 0) > 0).map((resource) => ({
      resource,
      amount: amounts![resource]!,
    }));
</script>

<script lang="ts">
  type BuildingSlotProps = {
    slot: SlotView;
    /** The colour the building's mark takes: its owner's. */
    tint: number;
    chain?: ConceptChain;
    /** Why the slot's actions cannot be taken, as their tooltip; `null` lets them be taken. */
    unavailable?: string | null;
    figures?: SlotFigures;
    /** Order one of the free slot's options, by its template. */
    onBuild?: (template: string) => void;
    onUpgrade?: () => void;
    onDemolish?: () => void;
    onCancel?: () => void;
  };
  const {
    slot,
    tint,
    chain,
    unavailable = null,
    figures = {},
    onBuild,
    onUpgrade,
    onDemolish,
    onCancel,
  }: BuildingSlotProps = $props();

  const building = $derived(
    slot.state === "built" || slot.state === "construction"
      ? slot.building
      : slot.state === "foreign"
        ? slot.building
        : null,
  );
  const drawing = $derived(building && buildingDrawing(building.sprite, building.level));
  /** The share done, as the map's construction overlay computes it. */
  const progress = $derived(slot.state === "construction" ? constructionProgress(slot) : 0);
  const options = $derived(figures.options ?? []);
  const halted = $derived(slot.state === "built" && (slot.building.state & BuildingState.Halted) !== 0);
  const upgrading = $derived(slot.state === "built" && (slot.building.state & BuildingState.Upgrading) !== 0);
</script>

{#snippet action(
  label: string | undefined,
  icon: IconName | undefined,
  variant: "primary" | "secondary" | "ghost",
  onclick?: () => void,
  extra = "",
)}
  {#if unavailable || !onclick}
    <Tooltip content={unavailable ?? NOT_YET}>
      <Button
        size="sm"
        {variant}
        {icon}
        class={extra}
        aria-label={label ? undefined : (unavailable ?? NOT_YET)}
        disabled>{label}</Button
      >
    </Tooltip>
  {:else}
    <Button size="sm" {variant} {icon} class={extra} {onclick}>{label}</Button>
  {/if}
{/snippet}

<!-- figures by resource, signed as changes per turn or as plain quantities -->
{#snippet amounts(figures: ResourceAmounts | undefined, sign: 1 | -1 | 0)}
  {#each named(figures) as { resource, amount } (resource)}
    <span class="inline-flex items-center gap-0.5">
      {sign === 0 ? formatQuantity(amount) : formatChange(sign * amount)}<Icon
        name={resource}
        size="xs"
        style="color: var(--color-resource-{resource})"
      />
    </span>
  {/each}
{/snippet}

{#snippet pips(level: number, levels: number)}
  <!-- one pip per level its template has: scripts decide how many there are -->
  <span class="flex gap-[3px]" aria-label="Level {level} of {levels}">
    {#each Array.from({ length: levels }, (_, index) => index + 1) as step (step)}
      <i class={["pip", step <= level && "on"]}></i>
    {/each}
  </span>
{/snippet}

<div class="flex flex-col gap-2.5">
  <div class="flex items-center gap-3.5">
    <!-- the socket: the slot drawn as the map draws a tile -->
    <div class="relative shrink-0" style:width="{SOCKET.width}px" style:height="{SOCKET.height}px">
      <svg
        class="absolute inset-0 size-full overflow-visible"
        viewBox="0 0 100 115.5"
        preserveAspectRatio="none"
        aria-hidden="true"
      >
        {#if slot.state === "free"}
          <polygon points={HEX} class="free" />
        {:else if slot.state === "construction"}
          <polygon points={HEX} class="busy" />
          <polygon points={HEX} class="track" />
          <polygon points={HEX} class="ring" pathLength="100" stroke-dasharray="{100 * progress} 100" />
        {:else if slot.state === "built"}
          <polygon points={HEX} class="built" />
        {:else}
          <polygon points={HEX} class="none" />
        {/if}
      </svg>
      {#if drawing && slot.state !== "foreign"}
        <EntityPortrait
          sprite={drawing.sprite}
          {tint}
          width={ART.width}
          height={ART.height}
          ghost={slot.state === "construction"}
          class="absolute -top-[7px] -left-[6px]"
        />
      {/if}
      <div class="absolute inset-0 flex flex-col items-center justify-center">
        {#if slot.state === "free"}
          <Icon name="plus" size="xl" class="text-accent" />
        {:else if slot.state === "construction"}
          <span class="font-display text-[34px] leading-[30px] font-bold text-fg tabular-nums">{slot.turnsLeft}</span>
          <span class="font-display text-[10px] font-semibold tracking-[.12em] text-fg-subtle uppercase">
            {slot.turnsLeft === 1 ? "turn" : "turns"}
          </span>
        {:else if slot.state === "foreign"}
          <Icon name="lock" size="lg" class="text-fg-subtle" />
        {:else if slot.state === "none"}
          <Icon name="unavailable" size="lg" class="text-fg-subtle" />
        {:else if slot.state === "settlement"}
          <Icon name="settlement" size="lg" class="text-fg-subtle" />
        {/if}
      </div>
    </div>

    <div class="flex min-w-0 flex-col gap-1">
      <div class="font-display text-[11px] font-semibold tracking-[.08em] text-fg-subtle uppercase">Building slot</div>
      <div class="flex items-center gap-2">
        <b class="font-display text-[17px] font-semibold tracking-wide text-fg uppercase">
          {#if slot.state === "free"}Free slot
          {:else if slot.state === "construction" || slot.state === "built"}{slot.building.name}
          {:else if slot.state === "foreign"}Not yours
          {:else if slot.state === "none"}No slot
          {:else}Settlement{/if}
        </b>
        {#if slot.state === "free"}
          <Badge tone="accent" size="sm">Buildable</Badge>
        {:else if slot.state === "construction"}
          <Badge tone="accent" size="sm">Under construction</Badge>
        {:else if slot.state === "built"}
          {@render pips(slot.building.level, slot.building.levels)}
          {#if halted}
            <Badge tone="danger" size="sm">Halted</Badge>
          {:else if upgrading}
            <Badge tone="accent" size="sm">Upgrading</Badge>
          {/if}
        {:else if slot.state === "foreign"}
          <Badge tone="danger" size="sm">{slot.owner.name}</Badge>
        {:else if slot.state === "settlement"}
          <Badge size="sm">{slot.settlement}</Badge>
        {/if}
      </div>
      <p class="m-0 text-[13px] leading-[18px] text-fg-muted">
        {#if slot.state === "free"}
          One building fits this tile.
        {:else if slot.state === "construction"}
          Level {slot.building.level + 1} under way.{figures.endsTurn
            ? ` Completes at the end of turn ${figures.endsTurn}.`
            : ""}
        {:else if slot.state === "built"}
          Level {slot.building.level}{slot.building.level >= slot.building.levels ? ", the highest." : "."}
        {:else if slot.state === "foreign"}
          Inside the borders of {slot.owner.name}: only a tile's owner builds on it.
        {:else if slot.state === "none"}
          {slot.reason}
        {:else}
          Its buildings belong to the <ConceptTerm id="settlement" text="settlement" {chain} />, which this panel does
          not manage yet.
        {/if}
      </p>
    </div>
  </div>

  {#if slot.state === "free" || slot.state === "construction" || slot.state === "built" || slot.state === "foreign"}
    <div
      class="flex flex-col gap-1.5 rounded-tl-cut-md rounded-br-cut-md border border-line bg-surface-sunken p-2 text-sm chamfer"
    >
      {#if slot.state === "free"}
        <div class="font-display text-[11px] font-semibold tracking-[.08em] text-fg-subtle uppercase">Build here</div>
        {#if options.length === 0}
          <p class="m-0 py-1 text-xs text-fg-subtle">Nothing the nation can build suits this tile.</p>
        {/if}
        {#each options as option (option.template)}
          {@const art = buildingDrawing(option.sprite, 1)}
          <div class="flex items-center gap-2.5 py-1">
            {#if art}
              <EntityPortrait sprite={art.sprite} {tint} width={34} height={39} class="shrink-0" />
            {/if}
            <div class="flex min-w-0 flex-1 flex-col">
              <b class="font-display text-sm font-semibold tracking-wide text-fg uppercase"
                >{buildingName(option.type)}</b
              >
              {#if option.cost !== undefined}
                <span class="flex flex-wrap items-center gap-x-1.5 text-xs text-fg-subtle tabular-nums">
                  {@render amounts(option.cost, 0)}
                  {#if option.turns !== undefined}<span>· {option.turns} {option.turns === 1 ? "turn" : "turns"}</span
                    >{/if}
                </span>
              {/if}
            </div>
            {#if option.favoured}
              <Badge tone="success" size="sm">{option.favoured}</Badge>
            {/if}
            {@render action(
              "Build",
              undefined,
              option.favoured ? "primary" : "secondary",
              onBuild && (() => onBuild(option.template)),
            )}
          </div>
        {/each}
      {:else if slot.state === "construction"}
        <div class="flex justify-between gap-4">
          <span>Turns left</span><span class="num text-fg">{slot.turnsLeft} of {slot.turnsTotal}</span>
        </div>
        <div class="h-1 bg-line">
          <i class="block h-full bg-linear-to-r from-[#8ea2ff] to-accent" style:width="{100 * progress}%"></i>
        </div>
        {#if named(figures.spent).length > 0}
          <div class="flex justify-between gap-4">
            <span>Spent</span><span class="num flex items-center gap-1.5 text-fg"
              >{@render amounts(figures.spent, 0)}</span
            >
          </div>
        {/if}
        {#if named(figures.refund).length > 0}
          <div class="flex justify-between gap-4">
            <span>Refund</span><span class="num flex items-center gap-1.5 text-fg"
              >{@render amounts(figures.refund, 0)}</span
            >
          </div>
        {/if}
        <div class="mt-1 flex">
          {@render action("Cancel", "close", "ghost", onCancel, "text-danger!")}
        </div>
      {:else if slot.state === "built"}
        {#if halted}
          <p class="m-0 text-xs text-danger">Its upkeep could not be paid: it makes nothing until it is.</p>
        {/if}
        {#if named(figures.output).length > 0}
          <div class="flex justify-between gap-4">
            <span>Output</span>
            <span class={["num flex items-center gap-1.5", halted ? "text-fg-subtle line-through" : "text-success"]}>
              {@render amounts(figures.output, 1)} / turn
            </span>
          </div>
        {/if}
        {#if named(figures.upkeep).length > 0}
          <div class="flex justify-between gap-4">
            <ConceptTerm id="upkeep" {chain} />
            <span class="num flex items-center gap-1.5 text-danger">{@render amounts(figures.upkeep, -1)} / turn</span>
          </div>
        {/if}
        {#if upgrading}
          <div class="flex justify-between gap-4">
            <span>Upgrading to level {slot.building.level + 1}</span>
            <span class="num text-fg">{slot.building.turnsLeft} of {slot.building.turnsTotal} turns</span>
          </div>
          {#if figures.endsTurn}
            <p class="m-0 text-xs text-fg-subtle">Completes at the end of turn {figures.endsTurn}.</p>
          {/if}
        {:else if figures.upgrade}
          <div class="flex justify-between gap-4">
            <span>Level {slot.building.level + 1}</span>
            <span class="num flex flex-wrap items-center justify-end gap-x-1.5 text-fg">
              {@render amounts(figures.upgrade.cost, 0)}<span>· {figures.upgrade.turns} t</span>
            </span>
          </div>
        {/if}
        <div class="mt-1 flex items-center gap-1.5">
          {#if !upgrading && slot.building.level < slot.building.levels}
            {@render action(`Level ${slot.building.level + 1}`, "upgrade", "primary", onUpgrade)}
          {/if}
          {@render action(undefined, "demolish", "ghost", onDemolish, "text-danger!")}
        </div>
      {:else if slot.state === "foreign"}
        <div class="flex justify-between gap-4">
          <span>Owner</span>
          <span class="num flex items-center gap-1.5 text-fg">
            <i
              class="inline-block size-2.5 rounded-[2px]"
              style:background="#{slot.owner.color.toString(16).padStart(6, '0')}"
            ></i>
            {slot.owner.name}
          </span>
        </div>
        {#if slot.building}
          <div class="flex justify-between gap-4">
            <span>Their building</span><span class="num text-fg"
              >{slot.building.name} · {slot.building.level === 0 ? "being built" : `lvl ${slot.building.level}`}</span
            >
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .num {
    font: 600 15px/22px var(--font-display);
    font-variant-numeric: tabular-nums;
  }
  polygon.free {
    fill: var(--color-surface-sunken);
    stroke: color-mix(in oklab, var(--color-accent) 70%, transparent);
    stroke-width: 2;
    stroke-dasharray: 6 5;
  }
  polygon.busy {
    fill: var(--color-surface-overlay);
  }
  polygon.track {
    fill: none;
    stroke: color-mix(in oklab, var(--color-fg) 14%, transparent);
    stroke-width: 4;
  }
  polygon.ring {
    fill: none;
    stroke: var(--color-accent);
    stroke-width: 4;
    stroke-linecap: round;
    filter: drop-shadow(0 0 3px var(--color-accent));
  }
  polygon.built {
    fill: var(--color-surface-raised);
    stroke: var(--color-line-strong);
    stroke-width: 1.5;
  }
  polygon.none {
    fill: var(--color-surface-sunken);
    stroke: var(--color-line);
    stroke-width: 1.5;
    stroke-dasharray: 3 4;
  }
  .pip {
    width: 9px;
    height: 9px;
    clip-path: polygon(50% 0, 100% 25%, 100% 75%, 50% 100%, 0 75%, 0 25%);
    background: var(--color-line-strong);
  }
  .pip.on {
    background: var(--color-accent);
  }
</style>
