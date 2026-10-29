<script lang="ts" module>
  export const iconSizes = { xs: 12, sm: 14, md: 16, lg: 20, xl: 24 };
</script>

<script lang="ts">
  import type { LucideProps } from "@lucide/svelte";

  import { icons, type IconName } from "./icons";

  type IconProps = Omit<LucideProps, "size"> & {
    name: IconName;
    size?: keyof typeof iconSizes;
    /** Accessible name. Without one, the icon is decorative and hidden from assistive technologies. */
    label?: string;
  };
  const { name, size = "md", label, ...rest }: IconProps = $props();

  const Drawing = $derived(icons[name]);
</script>

<!-- Colored by `currentColor`: set a `text-*` class on the icon or its parent. -->
<Drawing
  size={iconSizes[size]}
  strokeWidth={1.75}
  role={label ? "img" : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : true}
  {...rest}
/>
