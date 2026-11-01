<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import { fn } from "storybook/test";

  import TilePanel from "./TilePanel.svelte";
  import { HILLS, SPARSE, YOU } from "./tileSamples";

  const { Story } = defineMeta({
    title: "Game/Tile/TilePanel",
    component: TilePanel,
    tags: ["autodocs"],
    args: { viewer: YOU.nation, onDetails: fn(), onClose: fn() },
    parameters: { layout: "centered" },
  });
</script>

<Story name="Your tile" args={{ details: HILLS }} />

<Story name="Out of sight" args={{ details: SPARSE }} />
