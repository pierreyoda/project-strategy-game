import type { StorybookConfig } from "@storybook/svelte-vite";

const config: StorybookConfig = {
  stories: ["../src/**/*.mdx", "../src/**/*.stories.@(ts|svelte)"],
  addons: ["@storybook/addon-svelte-csf", "@storybook/addon-docs", "@storybook/addon-a11y"],
  framework: "@storybook/svelte-vite",
  core: {
    builder: {
      name: "@storybook/builder-vite",
      // Electron Forge has no root `vite.config.ts`: reuse the renderer's one, for its Svelte and Tailwind CSS plugins.
      options: { viteConfigPath: "vite.renderer.config.ts" },
    },
  },
};

export default config;
