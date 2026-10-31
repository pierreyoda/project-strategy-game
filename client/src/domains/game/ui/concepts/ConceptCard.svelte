<script lang="ts">
  import { autoUpdate, computePosition, flip, offset, shift, type Placement } from "@floating-ui/dom";
  import type { Snippet } from "svelte";

  import Badge from "../../../../shared/ui/Badge.svelte";
  import Icon from "../../../../shared/ui/Icon.svelte";
  import type { Concept } from "../../concepts";
  import { type ConceptChain, MAX_CONCEPT_DEPTH } from "./chain.svelte";
  import ConceptLink from "./ConceptLink.svelte";

  type ConceptCardProps = {
    concept: Concept;
    /** Place in the chain, from 0. */
    depth?: number;
    /** Share of the dwell time elapsed, shown by the bar on the right; 1 once `open`. */
    dwell?: number;
    /** Whether the pointer may come in and the links respond. */
    open?: boolean;
    /** Opened by keyboard: the hints speak of keys. */
    byKeyboard?: boolean;
    /** A figure beside the title - "128 stored" - in place of the concept's kind. */
    meta?: string;
    /** The chain the card belongs to. Without one, the card stands still, as in a story. */
    chain?: ConceptChain;
    /**
     * What the card hangs from. With an anchor, the card renders in the top layer beside it; without
     * one, it is laid out where it is written.
     */
    anchor?: HTMLElement | null;
    placement?: Placement;
    /** More of the card, under the summary: a breakdown of figures, as the tooltips show. */
    children?: Snippet;
    class?: string;
  };
  const {
    concept,
    depth = 0,
    dwell = 1,
    open = true,
    byKeyboard = false,
    meta,
    chain,
    anchor = null,
    placement = "right-start",
    children,
    class: className,
  }: ConceptCardProps = $props();

  const id = $props.id();
  /** The last card of a chain: its words lead nowhere. */
  const last = $derived(depth >= MAX_CONCEPT_DEPTH - 1);

  let card: HTMLDivElement;

  // a popover renders in the top layer, out of reach of any parent's overflow clipping and stacking context
  $effect(() => {
    const from = anchor;
    if (!from) {
      return;
    }
    card.showPopover();
    // a card opened from a link hangs beside the card holding the link, on the link's line: beside the
    // link itself it would cover its parent, dwell bar and all, which is the way back
    const parent = depth > 0 ? from.closest<HTMLElement>("[popover]") : null;
    const reference = parent
      ? {
          getBoundingClientRect: () => {
            const { left, right } = parent.getBoundingClientRect();
            const { top, bottom, height } = from.getBoundingClientRect();
            return { x: left, y: top, left, right, top, bottom, width: right - left, height };
          },
        }
      : from;
    const stopUpdates = autoUpdate(from, card, async () => {
      const { x, y } = await computePosition(reference, card, {
        placement,
        strategy: "fixed",
        // 24 px from what opened it; flipped to the other side rather than pushed off the edge
        middleware: [offset(24), flip(), shift({ padding: 8 })],
      });
      card.style.left = `${x}px`;
      card.style.top = `${y}px`;
    });
    return () => {
      stopUpdates();
      card.hidePopover();
    };
  });

  // a card opened from the keyboard takes the focus, so that Tab reaches its links
  $effect(() => {
    if (byKeyboard && open) {
      card.focus();
    }
  });
</script>

<div
  bind:this={card}
  {id}
  popover={anchor ? "manual" : undefined}
  role="tooltip"
  tabindex="-1"
  aria-label={concept.title}
  class={[
    "relative m-0 w-80 rounded-tl-cut-md rounded-br-cut-md border bg-surface-overlay py-2.5 pr-5.5 pl-3 text-sm text-fg-muted shadow-overlay chamfer",
    "transition-[border-color,box-shadow] duration-200 outline-none",
    anchor && "fixed top-0 left-0 starting:open:opacity-0",
    open ? "border-accent/55 ring-1 ring-accent/20" : "border-line-strong",
    className,
  ]}
  onpointerenter={() => chain?.enterCard(depth)}
  onpointerleave={() => chain?.leaveCard(depth)}
  onfocusin={() => chain?.enterCard(depth)}
  onfocusout={(event) => {
    // focus moving within the card is not leaving it
    if (!(event.relatedTarget instanceof Node && card.contains(event.relatedTarget))) {
      chain?.leaveCard(depth);
    }
  }}
>
  <div class="mb-1.5 flex items-center justify-between gap-4">
    <span class="flex items-center gap-2 font-display font-semibold tracking-wider text-fg uppercase">
      {#if concept.icon}
        <Icon name={concept.icon} size="sm" />
      {/if}
      {concept.title}
    </span>
    {#if meta}
      <span class="font-display text-[13px] font-semibold tracking-wide text-fg tabular-nums">{meta}</span>
    {:else if concept.kind}
      <Badge tone="accent" variant="outline" size="sm">{concept.kind}</Badge>
    {/if}
  </div>

  <p>
    {#each concept.summary as segment, index (index)}
      {#if "concept" in segment}
        <ConceptLink id={segment.concept} text={segment.text} {depth} {chain} plain={last || !open} />
      {:else}
        {segment.text}
      {/if}
    {/each}
  </p>

  {@render children?.()}

  {#if open}
    <div
      class="mt-2 flex flex-wrap items-center gap-1.5 border-t border-dashed border-line pt-1.5 pr-10 text-xs text-fg-subtle"
    >
      {#if byKeyboard}
        <kbd class="kbd">Tab</kbd> next concept · <kbd class="kbd">Enter</kbd> open · <kbd class="kbd">Esc</kbd> close
      {:else if last}
        The chain ends here
      {:else}
        <Icon name="chevron-right" size="xs" />
        Move in to explore a concept
      {/if}
    </div>
  {/if}

  <!-- the dwell bar: the time the pointer has rested on the card's owner, before the card can be entered -->
  <div class="pointer-events-none absolute top-2 right-[7px] bottom-2 w-[3px] rounded-sm bg-fg/10" aria-hidden="true">
    <div
      class={[
        "absolute inset-x-0 top-0 rounded-sm bg-linear-to-b from-[#8ea2ff] via-accent via-55% to-[#9ff0ff]",
        "shadow-[0_0_6px_--alpha(var(--color-accent)/70%),0_0_1px_var(--color-accent)]",
        open && "animate-dwell-ready",
      ]}
      style:height="{Math.round(dwell * 100)}%"
    ></div>
  </div>

  <div
    class="absolute right-6.5 bottom-1.5 font-display text-[10px] font-semibold tracking-widest text-fg-subtle tabular-nums"
    aria-hidden="true"
  >
    {depth + 1} / {MAX_CONCEPT_DEPTH}{last ? " · last" : ""}
  </div>
</div>

<style>
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
