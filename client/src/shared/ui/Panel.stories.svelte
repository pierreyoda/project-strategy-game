<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";

  import Badge from "./Badge.svelte";
  import Button from "./Button.svelte";
  import Icon from "./Icon.svelte";
  import Panel from "./Panel.svelte";

  const { Story } = defineMeta({
    title: "UI/Panel",
    component: Panel,
    tags: ["autodocs"],
    argTypes: {
      variant: { control: "inline-radio", options: ["solid", "glass", "inset"] },
      padding: { control: "inline-radio", options: ["none", "sm", "md"] },
    },
    args: {
      title: "Settlement",
      class: "w-80",
    },
  });
</script>

{#snippet yields()}
  <dl class="grid grid-cols-[1fr_auto] items-center gap-x-2 gap-y-1.5 text-sm">
    <dt class="flex items-center gap-2 text-fg-muted"><Icon name="food" class="text-resource-food" />Food</dt>
    <dd class="text-right font-display font-semibold tabular-nums">+6</dd>
    <dt class="flex items-center gap-2 text-fg-muted"><Icon name="water" class="text-resource-water" />Water</dt>
    <dd class="text-right font-display font-semibold tabular-nums">+3</dd>
    <dt class="flex items-center gap-2 text-fg-muted"><Icon name="metals" class="text-resource-metals" />Metals</dt>
    <dd class="text-right font-display font-semibold text-danger tabular-nums">−2</dd>
  </dl>
{/snippet}

<Story name="Solid">
  {@render yields()}
</Story>

<Story name="With actions and footer" asChild>
  <Panel title="Settlement" class="w-80">
    {#snippet actions()}
      <Badge tone="success">Growing</Badge>
      <Button variant="ghost" size="sm" icon="close" aria-label="Close" />
    {/snippet}
    {@render yields()}
    {#snippet footer()}
      <Button size="sm" icon="settlement">Manage</Button>
    {/snippet}
  </Panel>
</Story>

<Story name="Glass" args={{ variant: "glass", title: "Tile 12, −4" }}>
  {#snippet template(args)}
    <!-- stands in for the map -->
    <div class="bg-[repeating-linear-gradient(60deg,#173a2a_0_24px,#1d4a8a_24px_48px,#7a5a2c_48px_72px)] p-8">
      <Panel {...args}>{@render yields()}</Panel>
    </div>
  {/snippet}
</Story>

<Story name="Inset" asChild>
  <Panel title="Stockpile" class="w-80">
    <Panel variant="inset" padding="sm">{@render yields()}</Panel>
  </Panel>
</Story>
