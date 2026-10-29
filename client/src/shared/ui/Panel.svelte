<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLAttributes } from "svelte/elements";

  type PanelProps = HTMLAttributes<HTMLElement> & {
    title?: string;
    /** `glass` lets the map show through, `inset` is a well nested in another panel. */
    variant?: "solid" | "glass" | "inset";
    padding?: "none" | "sm" | "md";
    /** Header controls, after the title. */
    actions?: Snippet;
    footer?: Snippet;
    children?: Snippet;
  };
  const {
    title,
    variant = "solid",
    padding = "md",
    actions,
    footer,
    class: className,
    children,
    ...rest
  }: PanelProps = $props();

  const id = $props.id();
</script>

<section
  aria-labelledby={title ? `${id}-title` : undefined}
  class={[
    "flex flex-col border chamfer",
    {
      solid: "rounded-tl-cut-lg rounded-br-cut-lg border-line bg-surface",
      glass: "rounded-tl-cut-lg rounded-br-cut-lg border-line bg-surface/80 backdrop-blur-md",
      inset: "rounded-tl-cut-md rounded-br-cut-md border-line bg-surface-sunken",
    }[variant],
    className,
  ]}
  {...rest}
>
  {#if title || actions}
    <header class="flex min-h-9 items-center gap-2 border-b border-line py-1 pr-1.5 pl-3">
      {#if title}
        <h2 id="{id}-title" class="font-display text-sm font-semibold tracking-wider text-fg-muted uppercase">
          {title}
        </h2>
      {/if}
      {#if actions}
        <div class="ml-auto flex items-center gap-1">{@render actions()}</div>
      {/if}
    </header>
  {/if}
  <div class={["grow", { none: "", sm: "p-2", md: "p-3" }[padding]]}>
    {@render children?.()}
  </div>
  {#if footer}
    <footer class="flex items-center gap-2 border-t border-line px-3 py-2">{@render footer()}</footer>
  {/if}
</section>
