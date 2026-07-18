/**
 * Discovers `src/plugins/*.{client,server}.{ts,tsx,js,jsx}` and wires them:
 * - server → Nitro `server.plugins` (via `discoverAppPlugins().server`)
 * - client → `virtual:app-plugins-client` (imported from `entry-client.tsx`)
 *
 * Analogous to Nuxt's `plugins/` scan + `.client` / `.server` suffixes.
 * Unsuffixed files are ignored. Non-recursive. Alphabetical order.
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { Plugin } from "vite";

const CLIENT_RE = /\.client\.(tsx?|jsx?)$/;
const SERVER_RE = /\.server\.(tsx?|jsx?)$/;

const VIRTUAL_ID = "virtual:app-plugins-client";
const RESOLVED_VIRTUAL_ID = `\0${VIRTUAL_ID}`;

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export type DiscoveredAppPlugins = {
  client: string[];
  server: string[];
};

export function discoverAppPlugins(
  pluginsDir: string = path.join(packageRoot, "src", "plugins"),
  root: string = packageRoot,
): DiscoveredAppPlugins {
  if (!fs.existsSync(pluginsDir)) {
    return { client: [], server: [] };
  }

  const names = fs.readdirSync(pluginsDir).sort();
  const client: string[] = [];
  const server: string[] = [];

  for (const name of names) {
    const abs = path.join(pluginsDir, name);
    if (!fs.statSync(abs).isFile()) continue;

    const rel = `./${path.relative(root, abs).split(path.sep).join("/")}`;
    if (CLIENT_RE.test(name)) {
      client.push(rel);
      continue;
    }
    if (SERVER_RE.test(name)) {
      server.push(rel);
    }
  }

  return { client, server };
}

/** @internal exported for tests */
export function generateClientModule(clientPaths: string[], root: string): string {
  if (clientPaths.length === 0) {
    return "export {};\n";
  }

  // Relative imports + no top-level await: absolute paths and `await setup()` were
  // getting dropped from Vinxi's production client graph (Faro never shipped).
  const lines: string[] = [];
  clientPaths.forEach((rel, i) => {
    const abs = path.resolve(root, rel);
    let relImport = path.relative(root, abs).split(path.sep).join("/");
    if (!relImport.startsWith(".")) relImport = `./${relImport}`;
    lines.push(`import p${i} from ${JSON.stringify(relImport)};`);
  });
  lines.push("");
  clientPaths.forEach((_, i) => {
    lines.push(`void Promise.resolve(p${i}.setup({})).catch((err) => console.error("[app-plugin]", err));`);
  });
  lines.push("");
  return lines.join("\n");
}

/** Vite plugin: resolve/load `virtual:app-plugins-client` and watch the plugins dir. */
export function appPlugins(): Plugin {
  return {
    name: "app-plugins",
    resolveId(id) {
      if (id === VIRTUAL_ID) return RESOLVED_VIRTUAL_ID;
    },
    load(id) {
      if (id !== RESOLVED_VIRTUAL_ID) return;
      const { client } = discoverAppPlugins();
      return generateClientModule(client, packageRoot);
    },
    configureServer(server) {
      const pluginsDir = path.join(packageRoot, "src", "plugins");
      server.watcher.add(pluginsDir);
      const invalidate = (file: string) => {
        if (!CLIENT_RE.test(file)) return;
        const mod = server.moduleGraph.getModuleById(RESOLVED_VIRTUAL_ID);
        if (!mod) return;
        server.moduleGraph.invalidateModule(mod);
      };
      server.watcher.on("add", invalidate);
      server.watcher.on("unlink", invalidate);
      server.watcher.on("change", invalidate);
    },
  };
}
