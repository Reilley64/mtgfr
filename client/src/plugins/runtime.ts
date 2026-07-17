/**
 * Nuxt-shaped app plugin constructors for SolidStart.
 *
 * - `defineClientPlugin` — browser boot (auto-loaded from `*.client.ts`)
 * - `defineServerPlugin` — Nitro process boot (auto-loaded from `*.server.ts`)
 *
 * Not supported (yet): dependsOn, parallel, provide, or a rich app context.
 * `ClientPluginContext` is a stub reserved for later hooks.
 */

import { defineNitroPlugin } from "nitropack/runtime/plugin";
import type { NitroApp, NitroAppPlugin } from "nitropack/types";

/** Reserved for future client plugin hooks (Nuxt-style app context). */
export type ClientPluginContext = Record<string, never>;

export type ClientPlugin = {
  readonly __kind: "client";
  setup: (ctx: ClientPluginContext) => void | Promise<void>;
};

export function defineClientPlugin(setup: (ctx: ClientPluginContext) => void | Promise<void>): ClientPlugin {
  return { __kind: "client", setup };
}

/** Thin Nitro wrapper so authors share one vocabulary with client plugins. */
export function defineServerPlugin(setup: (nitroApp: NitroApp) => void): NitroAppPlugin {
  return defineNitroPlugin(setup);
}
