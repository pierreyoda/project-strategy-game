<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";

  import Icon, { iconSizes } from "./Icon.svelte";
  import { icons, type IconName } from "./icons";

  const { Story } = defineMeta({
    title: "UI/Icon",
    component: Icon,
    tags: ["autodocs"],
    argTypes: {
      name: { control: "select", options: Object.keys(icons) },
      size: { control: "inline-radio", options: Object.keys(iconSizes) },
    },
    args: {
      name: "food",
      size: "xl",
    },
  });

  const resources = [
    ["credits", "text-resource-credits"],
    ["water", "text-resource-water"],
    ["food", "text-resource-food"],
    ["electricity", "text-resource-electricity"],
    ["metals", "text-resource-metals"],
    ["oil", "text-resource-oil"],
    ["uranium", "text-resource-uranium"],
  ] as const;
</script>

<Story name="Default" />

<Story name="Resources" asChild>
  <div class="flex gap-6">
    {#each resources as [name, color] (name)}
      <div class="flex flex-col items-center gap-2">
        <Icon {name} size="xl" class={color} />
        <span class="font-display text-xs font-semibold tracking-wider text-fg-muted uppercase">{name}</span>
      </div>
    {/each}
  </div>
</Story>

<Story name="Gallery" asChild>
  <div class="grid grid-cols-[repeat(auto-fill,minmax(7rem,1fr))] gap-2">
    {#each Object.keys(icons) as IconName[] as name (name)}
      <div class="flex flex-col items-center gap-2 border border-line bg-surface p-3">
        <Icon {name} size="lg" />
        <code class="text-2xs text-fg-subtle">{name}</code>
      </div>
    {/each}
  </div>
</Story>

<Story name="Sizes" asChild>
  <div class="flex items-end gap-4">
    {#each Object.keys(iconSizes) as (keyof typeof iconSizes)[] as size (size)}
      <Icon name="electricity" {size} />
    {/each}
  </div>
</Story>
