<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";

  import Button from "./Button.svelte";
  import Icon from "./Icon.svelte";
  import Tooltip from "./Tooltip.svelte";

  const { Story } = defineMeta({
    title: "UI/Tooltip",
    component: Tooltip,
    tags: ["autodocs"],
    argTypes: {
      placement: { control: "select", options: ["top", "right", "bottom", "left"] },
    },
    parameters: {
      layout: "centered",
      // room for the open tooltips in the documentation page
      docs: { story: { height: "200px" } },
    },
  });
</script>

<Story name="Simple" args={{ content: "Ends your turn once every unit has orders.", open: true }}>
  <Button variant="primary" icon="turn">End turn</Button>
</Story>

<Story
  name="With title and hotkey"
  args={{ title: "Fortify", hotkey: "F", content: "+25% defense until moving.", open: true }}
>
  <Button icon="defense" aria-label="Fortify" />
</Story>

<Story name="Explaining a disabled control" args={{ content: "Requires 40 metals.", open: true }}>
  <Button icon="settlement" disabled>Build outpost</Button>
</Story>

{#snippet breakdown()}
  <dl class="grid grid-cols-[1fr_auto] gap-x-6 gap-y-1">
    <dt>Reactors</dt>
    <dd class="text-right text-success tabular-nums">+18</dd>
    <dt>Settlements</dt>
    <dd class="text-right text-danger tabular-nums">−4</dd>
  </dl>
{/snippet}

<Story name="Rich content" args={{ title: "Electricity", content: breakdown, placement: "bottom", open: true }}>
  <span class="inline-flex items-center gap-1.5 font-display text-lg font-semibold tabular-nums">
    <Icon name="electricity" class="text-resource-electricity" />
    128
    <span class="text-sm text-success">+14</span>
  </span>
</Story>
