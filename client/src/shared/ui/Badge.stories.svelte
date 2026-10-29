<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";

  import Badge from "./Badge.svelte";
  import { icons } from "./icons";

  const { Story } = defineMeta({
    title: "UI/Badge",
    component: Badge,
    tags: ["autodocs"],
    argTypes: {
      tone: { control: "inline-radio", options: ["neutral", "accent", "success", "warning", "danger"] },
      variant: { control: "inline-radio", options: ["soft", "solid", "outline"] },
      size: { control: "inline-radio", options: ["sm", "md"] },
      icon: { control: "select", options: Object.keys(icons) },
    },
  });
</script>

<Story name="Default">Idle</Story>

<Story name="With icon" args={{ tone: "warning", icon: "warning" }}>Low stock</Story>

<Story name="Counter" args={{ tone: "danger", variant: "solid", size: "sm" }}>3</Story>

<Story name="Tones and variants" asChild>
  <div class="grid grid-cols-[repeat(5,max-content)] items-center gap-3">
    {#each ["soft", "solid", "outline"] as const as variant (variant)}
      <Badge {variant}>Idle</Badge>
      <Badge {variant} tone="accent">Selected</Badge>
      <Badge {variant} tone="success">+12</Badge>
      <Badge {variant} tone="warning">Unrest</Badge>
      <Badge {variant} tone="danger">Besieged</Badge>
    {/each}
  </div>
</Story>
