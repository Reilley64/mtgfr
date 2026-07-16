// CVA beta (`cva@1.0.0-beta.*`) wired through our theme-aware tailwind-merge — same conflict rules as `cn`.
import { defineConfig } from "cva";
import { mergeTw } from "~/lib/cn";

export type { VariantProps } from "cva";

export const { cva, cx, compose } = defineConfig({
  hooks: {
    onComplete: (className) => mergeTw(className),
  },
});
