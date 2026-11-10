<script lang="ts" module>
  import Badge from "../../../shared/ui/Badge.svelte";
  import Button from "../../../shared/ui/Button.svelte";
  import Icon from "../../../shared/ui/Icon.svelte";
  import Panel from "../../../shared/ui/Panel.svelte";
  import {
    type CompanyState,
    UNIT_TYPE_NAMES,
    type UnitType,
    companyStates,
    describeStrength,
  } from "../../map/entities";
  import { nationColor } from "../nations";
  import type { SelectedUnit } from "../state/activeGame.svelte";

  const cssColor = (color: number) => `#${color.toString(16).padStart(6, "0")}`;

  /**
   * The counter's sign, small: the same WW2 shapes as the sprites, in a 64 x 44 card whose border
   * runs from 3 to 61 and 3 to 41 - the cross from corner to corner, the wheels and the track inside.
   */
  const SIGNS: Record<UnitType, string> = {
    infantry_division: "M3 3 L61 41 M3 41 L61 3",
    motorized_division:
      "M3 3 L61 41 M3 41 L61 3 M23.5 32 a2.5 2.5 0 1 0 5 0 a2.5 2.5 0 1 0 -5 0 M35.5 32 a2.5 2.5 0 1 0 5 0 a2.5 2.5 0 1 0 -5 0",
    light_tank_division: "M22.5 11.5 h19 a9 9 0 0 1 0 18 h-19 a9 9 0 0 1 0 -18 Z",
  };
  const isUnitType = (type: string | null): type is UnitType => type !== null && type in UNIT_TYPE_NAMES;
  /**
   * A company's circle, like the counter's on the map, on a dark disc: a lit core in a soft glow, a
   * ring in the warning colour, or a faint socket.
   */
  const COMPANY_CLASSES: Record<CompanyState, string> = {
    active: "after:bg-white shadow-[0_0_6px_1px_rgb(191_233_255/0.6)]",
    suppressed: "after:border-[2.5px] after:border-warning",
    lost: "after:bg-white/10",
  };
</script>

<script lang="ts">
  type UnitPanelProps = {
    selected: SelectedUnit;
    /** The nation being played, whose colour the glyph takes. */
    viewer: number;
    hasNext: boolean;
    onSkip: () => void;
    onNext: () => void;
  };
  const { selected, viewer, hasNext, onSkip, onNext }: UnitPanelProps = $props();

  const unit = $derived(selected.unit);
  const type = $derived(isUnitType(unit.type) ? unit.type : null);
  const companies = $derived(companyStates(unit.strength));
  const strength = $derived(describeStrength(unit.strength));
  const spent = $derived(unit.movementLeft === 0);
</script>

<Panel title="Unit" variant="glass" padding="sm" class="w-72">
  {#snippet actions()}
    {#if selected.skipped}
      <Badge>Skipped</Badge>
    {:else if spent}
      <Badge tone="warning">Spent</Badge>
    {:else}
      <Badge tone="accent">Ready</Badge>
    {/if}
  {/snippet}

  <div class="flex items-center gap-3">
    <svg viewBox="0 0 64 44" class="h-11 w-16 shrink-0 drop-shadow" role="img" aria-label="Counter">
      <defs>
        <clipPath id="unit-panel-sign"><path d="M6 3 H61 V37 L57 41 H3 V6 Z" /></clipPath>
      </defs>
      <path d="M6 0 H64 V38 L58 44 H0 V6 Z" fill={cssColor(nationColor(viewer))} />
      {#if type}
        <path d={SIGNS[type]} clip-path="url(#unit-panel-sign)" fill="none" stroke="white" stroke-width="2.5" />
      {/if}
      <path d="M6 3 H61 V37 L57 41 H3 V6 Z" fill="none" stroke="white" stroke-width="1.5" />
    </svg>
    <div class="min-w-0">
      <div class="truncate font-display text-base font-semibold tracking-wide uppercase">
        {type ? UNIT_TYPE_NAMES[type] : "Unknown unit"}
      </div>
      <div class="font-display text-xs text-fg-subtle tabular-nums">Designation {unit.id + 1}</div>
    </div>
  </div>

  <dl class="mt-3 grid grid-cols-[1fr_auto] items-center gap-x-2 gap-y-1.5 text-sm">
    <dt class="flex items-center gap-2 text-fg-muted"><Icon name="health" />Strength</dt>
    <dd class="flex max-w-36 flex-wrap justify-end gap-1" title={strength} aria-label={strength}>
      {#each companies as state, company (company)}
        <span
          class={[
            "relative size-3.5 rounded-full border border-line-strong bg-ground",
            "after:absolute after:inset-[2px] after:rounded-full after:content-['']",
            COMPANY_CLASSES[state],
          ]}
        ></span>
      {/each}
    </dd>
    <dt class="flex items-center gap-2 text-fg-muted"><Icon name="movement" />Movement</dt>
    <dd class={["text-right font-display font-semibold tabular-nums", spent && "text-fg-subtle"]}>
      {unit.movementLeft} / {unit.movement}
    </dd>
    <dt class="flex items-center gap-2 text-fg-muted"><Icon name="map" />Sight</dt>
    <dd class="text-right font-display font-semibold tabular-nums">{unit.sight}</dd>
  </dl>

  {#snippet footer()}
    <Button variant="ghost" size="sm" disabled={selected.skipped} onclick={onSkip}>Skip</Button>
    <Button size="sm" class="ml-auto" disabled={!hasNext} onclick={onNext}>Next unit</Button>
  {/snippet}
</Panel>
