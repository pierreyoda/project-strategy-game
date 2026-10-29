<script lang="ts">
  import { autoUpdate, computePosition, flip, offset, shift, type Placement } from "@floating-ui/dom";
  import type { Snippet } from "svelte";

  type TooltipProps = {
    title?: string;
    /** Plain text, or a snippet for rich content such as a breakdown of numbers. */
    content?: string | Snippet;
    /** Keyboard shortcut of the action, shown next to the title. */
    hotkey?: string;
    placement?: Placement;
    /** Hovering delay before showing up, in milliseconds. Keyboard focus shows it right away. */
    delay?: number;
    open?: boolean;
    /** The element described by the tooltip. */
    children: Snippet;
  };
  let {
    title,
    content,
    hotkey,
    placement = "top",
    delay = 400,
    open = $bindable(false),
    children,
  }: TooltipProps = $props();

  const id = $props.id();
  let anchor: HTMLSpanElement;
  let tooltip: HTMLDivElement;
  let showTimeout: ReturnType<typeof setTimeout> | undefined;

  function show(after: number) {
    clearTimeout(showTimeout);
    showTimeout = setTimeout(() => (open = true), after);
  }

  function hide() {
    clearTimeout(showTimeout);
    open = false;
  }

  $effect(() => () => clearTimeout(showTimeout));

  $effect(() => {
    const described = anchor.firstElementChild;
    described?.setAttribute("aria-describedby", id);
    return () => described?.removeAttribute("aria-describedby");
  });

  // a popover renders in the top layer, out of reach of any parent's overflow clipping and stacking context
  $effect(() => {
    if (!open) {
      return;
    }
    tooltip.showPopover();
    const stopUpdates = autoUpdate(anchor, tooltip, async () => {
      const { x, y } = await computePosition(anchor, tooltip, {
        placement,
        strategy: "fixed",
        middleware: [offset(8), flip(), shift({ padding: 8 })],
      });
      tooltip.style.left = `${x}px`;
      tooltip.style.top = `${y}px`;
    });
    return () => {
      stopUpdates();
      tooltip.hidePopover();
    };
  });
</script>

<svelte:window onkeydown={(event) => event.key === "Escape" && hide()} />

<!--
  Listening on a wrapper also catches hovering disabled controls, to explain why they are.
  It only reacts to hovering and focus: the described element stays the interactive one.
-->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<span
  bind:this={anchor}
  class="inline-flex"
  onpointerenter={() => show(delay)}
  onpointerleave={hide}
  onfocusin={() => show(0)}
  onfocusout={hide}
>
  {@render children()}
</span>

<div
  bind:this={tooltip}
  {id}
  popover="manual"
  role="tooltip"
  class={[
    "fixed top-0 left-0 m-0 w-max max-w-72 rounded-tl-cut-md rounded-br-cut-md border border-line-strong chamfer",
    "bg-surface-overlay px-3 py-2 text-sm text-fg-muted shadow-overlay",
    "transition-opacity duration-150 starting:open:opacity-0",
  ]}
>
  {#if title || hotkey}
    <div class="mb-1 flex items-center justify-between gap-4">
      {#if title}
        <span class="font-display font-semibold tracking-wider text-fg uppercase">{title}</span>
      {/if}
      {#if hotkey}
        <kbd
          class="rounded-sm border border-line-strong bg-surface-raised px-1.5 font-display text-2xs font-semibold text-fg-muted"
        >
          {hotkey}
        </kbd>
      {/if}
    </div>
  {/if}
  {#if typeof content === "string"}
    <p>{content}</p>
  {:else if content}
    {@render content()}
  {/if}
</div>
