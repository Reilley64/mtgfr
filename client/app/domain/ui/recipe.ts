// The one place cva meets this project's tailwind-merge config. Recipes import `cva` from here so
// every variant string is merged with THEME_SCALES knowledge — see domain/cn.ts for why stock
// tailwind-merge is wrong for our scales.

import { defineConfig } from "cva";
import { cn } from "../cn";

export const { cva } = defineConfig({ hooks: { onComplete: (className) => cn(className) } });
