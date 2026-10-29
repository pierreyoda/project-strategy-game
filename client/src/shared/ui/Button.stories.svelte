<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import { fn } from "storybook/test";

  import Button from "./Button.svelte";
  import { icons } from "./icons";

  const { Story } = defineMeta({
    title: "UI/Button",
    component: Button,
    tags: ["autodocs"],
    argTypes: {
      variant: { control: "inline-radio", options: ["primary", "secondary", "ghost", "danger"] },
      size: { control: "inline-radio", options: ["sm", "md", "lg"] },
      icon: { control: "select", options: Object.keys(icons) },
    },
    args: {
      onclick: fn(),
    },
  });
</script>

<Story name="Primary" args={{ variant: "primary", icon: "turn" }}>End turn</Story>

<Story name="Secondary">Cancel</Story>

<Story name="Ghost" args={{ variant: "ghost", icon: "chevron-right" }}>Details</Story>

<Story name="Danger" args={{ variant: "danger" }}>Disband army</Story>

<Story name="Disabled" args={{ icon: "research", disabled: true }}>Research</Story>

<Story name="Icon only" args={{ icon: "settings", "aria-label": "Settings" }} />

<Story name="Toggles" asChild>
  <div class="flex items-center gap-1">
    <Button variant="ghost" icon="map" aria-label="Terrain" aria-pressed="true" />
    <Button variant="ghost" icon="credits" aria-label="Resources" aria-pressed="false" />
    <Button variant="ghost" icon="attack" aria-label="Military" aria-pressed="false" />
  </div>
</Story>

<Story name="Variants and sizes" asChild>
  <div class="grid grid-cols-[repeat(4,max-content)] items-center gap-3">
    {#each ["sm", "md", "lg"] as const as size (size)}
      <Button {size} variant="primary" icon="turn">End turn</Button>
      <Button {size}>Cancel</Button>
      <Button {size} variant="ghost">Details</Button>
      <Button {size} variant="danger">Disband</Button>
    {/each}
  </div>
</Story>
