import js from "@eslint/js";
import tsPlugin from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";
import { defineConfig, globalIgnores } from "eslint/config";
import importPlugin from "eslint-plugin-import";
import prettierRecommended from "eslint-plugin-prettier/recommended";
import storybook from "eslint-plugin-storybook";
import svelte from "eslint-plugin-svelte";
import globals from "globals";

import svelteConfig from "./svelte.config.js";

export default defineConfig([
  globalIgnores([".vite/", "out/", "storybook-static/"]),
  js.configs.recommended,
  tsPlugin.configs["flat/recommended"],
  svelte.configs.recommended,
  storybook.configs["flat/recommended"],
  importPlugin.flatConfigs.recommended,
  importPlugin.flatConfigs.electron,
  importPlugin.flatConfigs.typescript,
  {
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    rules: {
      // Redundant with TypeScript's own resolution, and the node resolver ignores package.json "exports".
      "import/no-unresolved": "off",
    },
  },
  {
    files: ["**/*.svelte", "**/*.svelte.ts", "**/*.svelte.js"],
    languageOptions: {
      parserOptions: {
        parser: tsParser,
        extraFileExtensions: [".svelte"],
        svelteConfig,
      },
    },
  },
  // Must stay last: disables the stylistic rules conflicting with Prettier.
  prettierRecommended,
  svelte.configs.prettier,
]);
