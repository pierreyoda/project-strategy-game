<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLAttributes } from "svelte/elements";

  import Icon from "./Icon.svelte";
  import type { IconName } from "./icons";

  type BadgeProps = HTMLAttributes<HTMLSpanElement> & {
    tone?: "neutral" | "accent" | "success" | "warning" | "danger";
    variant?: "soft" | "solid" | "outline";
    size?: "sm" | "md";
    icon?: IconName;
    children?: Snippet;
  };
  const {
    tone = "neutral",
    variant = "soft",
    size = "md",
    icon,
    class: className,
    children,
    ...rest
  }: BadgeProps = $props();

  // spelled out in full for Tailwind CSS to find them
  const tones = {
    soft: {
      neutral: "bg-fg/10 text-fg-muted",
      accent: "bg-accent/15 text-accent",
      success: "bg-success/15 text-success",
      warning: "bg-warning/15 text-warning",
      danger: "bg-danger/15 text-danger",
    },
    solid: {
      neutral: "bg-fg-muted text-ground",
      accent: "bg-accent text-on-accent",
      success: "bg-success text-ground",
      warning: "bg-warning text-ground",
      danger: "bg-danger text-ground",
    },
    outline: {
      neutral: "border-line-strong text-fg-muted",
      accent: "border-accent/60 text-accent",
      success: "border-success/60 text-success",
      warning: "border-warning/60 text-warning",
      danger: "border-danger/60 text-danger",
    },
  };
</script>

<!-- Fully rounded bevels draw a stretched hexagon, echoing the map's tiles. -->
<span
  class={[
    "inline-flex items-center gap-1 rounded-full border border-transparent font-display font-semibold tracking-wider whitespace-nowrap uppercase tabular-nums chamfer",
    { sm: "h-4 px-2 text-2xs", md: "h-5 px-2.5 text-xs" }[size],
    tones[variant][tone],
    className,
  ]}
  {...rest}
>
  {#if icon}
    <Icon name={icon} size="xs" />
  {/if}
  {@render children?.()}
</span>
