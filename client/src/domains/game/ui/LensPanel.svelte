<script lang="ts" module>
  import Button from "../../../shared/ui/Button.svelte";
  import type { IconName } from "../../../shared/ui/icons";
  import Panel from "../../../shared/ui/Panel.svelte";
  import Tooltip from "../../../shared/ui/Tooltip.svelte";
  import { DEPOSIT_RESOURCES } from "../../map/deposits";
  import { nationCssColor } from "../nations";
  import { RESOURCE_NAMES } from "../resources";
  import { LENS_HOTKEYS, LENS_KINDS, LENS_NAMES, type Lens, type LensKind, type ResourceFilter } from "../state/lens";

  const LENS_ICONS: Record<LensKind, IconName> = {
    resources: "deposits",
    political: "territory",
  };

  const LENS_HINTS: Record<LensKind, string> = {
    resources: "Every deposit you know of, by resource and richness. Settlements and units are muted.",
    political: "Who holds which tiles, and how far each nation's influence reaches.",
  };
</script>

<script lang="ts">
  type LensPanelProps = {
    lens: Lens;
    /** The nations the political lens names, the viewer's first. */
    nations: readonly { id: number; name: string }[];
    /** The nation being played, marked in the legend. */
    viewer: number;
    onToggle: (kind: LensKind) => void;
    onFilter: (filter: ResourceFilter) => void;
  };
  const { lens, nations, viewer, onToggle, onFilter }: LensPanelProps = $props();
</script>

<Panel variant="glass" padding="sm" aria-label="Map lenses">
  <div class="flex items-center justify-between gap-3">
    <span class="pl-1 font-display text-2xs font-semibold tracking-wider text-fg-muted uppercase">Lens</span>
    <div class="flex items-center gap-1">
      {#each LENS_KINDS as kind (kind)}
        <Tooltip title={LENS_NAMES[kind]} content={LENS_HINTS[kind]} hotkey={LENS_HOTKEYS[kind]}>
          <Button
            variant="ghost"
            icon={LENS_ICONS[kind]}
            aria-label="{LENS_NAMES[kind]} lens"
            aria-pressed={lens.kind === kind}
            onclick={() => onToggle(kind)}
          />
        </Tooltip>
      {/each}
    </div>
  </div>

  {#if lens.kind === "resources"}
    <div class="mt-2 flex items-center gap-1 border-t border-line pt-2" role="group" aria-label="Resources shown">
      <Button
        variant="ghost"
        size="sm"
        class="aria-pressed:border-line-strong"
        aria-pressed={lens.filter === "all"}
        onclick={() => onFilter("all")}
      >
        All
      </Button>
      {#each DEPOSIT_RESOURCES as resource (resource)}
        <Tooltip
          title={RESOURCE_NAMES[resource]}
          content="Show {RESOURCE_NAMES[resource].toLowerCase()} deposits only."
        >
          <Button
            variant="ghost"
            size="sm"
            icon={resource}
            aria-label={RESOURCE_NAMES[resource]}
            aria-pressed={lens.filter === resource}
            class={[
              "aria-pressed:border-line-strong",
              { "opacity-50": lens.filter !== "all" && lens.filter !== resource },
            ]}
            style="color: var(--color-resource-{resource})"
            onclick={() => onFilter(lens.filter === resource ? "all" : resource)}
          />
        </Tooltip>
      {/each}
    </div>
  {:else if lens.kind === "political"}
    <ul class="mt-2 flex max-w-56 flex-col gap-1 border-t border-line pt-2 text-sm" aria-label="Nations">
      {#each nations as nation (nation.id)}
        <li class="flex items-center gap-2 px-1">
          <span class="size-3 shrink-0 rounded-xs" style="background: {nationCssColor(nation.id)}"></span>
          <span class="truncate">{nation.name}</span>
          {#if nation.id === viewer}
            <span class="ml-auto font-display text-2xs tracking-wider text-fg-muted uppercase">You</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</Panel>
