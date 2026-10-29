import { create } from "storybook/theming";

// Mirrors the design tokens of `src/index.css`.
export default create({
  base: "dark",
  brandTitle: "Project X",
  colorPrimary: "#3ec6e8",
  colorSecondary: "#3ec6e8",
  appBg: "#0f161f",
  appContentBg: "#090e14",
  appPreviewBg: "#090e14",
  appBorderColor: "#243243",
  barBg: "#0f161f",
  textColor: "#e2e9f1",
  textMutedColor: "#9aa8b8",
});
