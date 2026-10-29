<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  import Icon from "./Icon.svelte";
  import type { IconName } from "./icons";

  type ButtonProps = HTMLButtonAttributes & {
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "sm" | "md" | "lg";
    /** Leading icon. Without children, the button is a square icon button: give it an `aria-label`. */
    icon?: IconName;
    children?: Snippet;
  };
  const {
    variant = "secondary",
    size = "md",
    icon,
    type = "button",
    class: className,
    children,
    ...rest
  }: ButtonProps = $props();
</script>

<button
  {type}
  class={[
    "inline-flex shrink-0 items-center justify-center gap-2 rounded-tl-cut-md rounded-br-cut-md border chamfer",
    "font-display font-semibold tracking-wider whitespace-nowrap uppercase transition-colors select-none",
    "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
    "disabled:cursor-not-allowed disabled:opacity-40",
    {
      primary:
        "border-accent bg-accent text-on-accent enabled:hover:border-accent-hover enabled:hover:bg-accent-hover enabled:active:bg-accent-press",
      secondary:
        "border-line-strong bg-surface-raised text-fg enabled:hover:border-accent/60 enabled:hover:bg-surface-overlay enabled:active:bg-surface aria-pressed:border-accent aria-pressed:text-accent",
      ghost:
        "border-transparent text-fg-muted enabled:hover:bg-surface-raised enabled:hover:text-fg aria-pressed:bg-surface-raised aria-pressed:text-accent",
      danger:
        "border-danger-fill bg-danger-fill text-white enabled:hover:border-danger-fill-hover enabled:hover:bg-danger-fill-hover",
    }[variant],
    {
      sm: ["h-7 text-xs", children ? "px-2.5" : "w-7"],
      md: ["h-9 text-sm", children ? "px-4" : "w-9"],
      lg: ["h-11 text-base", children ? "px-5" : "w-11"],
    }[size],
    className,
  ]}
  {...rest}
>
  {#if icon}
    <Icon name={icon} {size} />
  {/if}
  {@render children?.()}
</button>
