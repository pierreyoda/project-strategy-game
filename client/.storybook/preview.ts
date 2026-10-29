import type { Preview } from "@storybook/svelte-vite";

import "../src/index.css";
import theme from "./theme";

const preview: Preview = {
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    backgrounds: {
      options: {
        ground: { name: "Ground", value: "var(--color-ground)" },
        surface: { name: "Surface", value: "var(--color-surface)" },
      },
    },
    docs: { theme },
  },
  initialGlobals: {
    backgrounds: { value: "ground" },
  },
};

export default preview;
