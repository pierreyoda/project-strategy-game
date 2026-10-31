<script lang="ts" module>
  import Badge from "../../../shared/ui/Badge.svelte";
  import Button from "../../../shared/ui/Button.svelte";
  import Icon from "../../../shared/ui/Icon.svelte";
  import Panel from "../../../shared/ui/Panel.svelte";
  import { STRENGTH_STEPS, UNIT_TYPE_NAMES, type UnitType, strengthOf } from "../../map/entities";
  import { nationColor } from "../nations";
  import type { SelectedUnit } from "../state/activeGame.svelte";

  const cssColor = (color: number) => `#${color.toString(16).padStart(6, "0")}`;

  /** The counter's glyph, small: the same APP-6 shapes as the sprites, in a 64 x 44 card. */
  const GLYPHS: Record<UnitType, string> = {
    infantry_division: "M14 10 L50 34 M14 34 L50 10",
    motorized_division: "M14 10 L50 34 M14 34 L50 10 M32 8 L32 36",
    light_tank_division: "M20 20 a12 7 0 1 0 24 0 a12 7 0 1 0 -24 0 M29 30 v6 h6",
  };
  const isUnitType = (type: string | null): type is UnitType => type !== null && type in UNIT_TYPE_NAMES;
  const STEPS = [...Array(STRENGTH_STEPS).keys()];
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
  const strength = $derived(strengthOf(unit.health));
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
      <path d="M6 0 H64 V38 L58 44 H0 V6 Z" fill={cssColor(nationColor(viewer))} />
      <path d="M6 3 H61 V37 L57 41 H3 V6 Z" fill="none" stroke="white" stroke-opacity="0.9" stroke-width="1.5" />
      {#if type}
        <path d={GLYPHS[type]} fill="none" stroke="white" stroke-width="2.5" stroke-linecap="round" />
      {/if}
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
    <dd class="flex items-center gap-1" aria-label="{strength.held} of {STRENGTH_STEPS} steps">
      {#each STEPS as step (step)}
        <span
          class={[
            "h-2 w-5 rounded-tl-cut-sm rounded-br-cut-sm bg-fg chamfer",
            step < strength.held ? "opacity-95" : "opacity-25",
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
