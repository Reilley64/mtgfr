import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { discoverAppPlugins } from "./app-plugins";

const tempDirs: string[] = [];

afterEach(() => {
  for (const dir of tempDirs.splice(0)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

function makeFixture(files: Record<string, string>): { root: string; pluginsDir: string } {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "app-plugins-"));
  tempDirs.push(root);
  const pluginsDir = path.join(root, "src", "plugins");
  fs.mkdirSync(pluginsDir, { recursive: true });
  for (const [name, body] of Object.entries(files)) {
    fs.writeFileSync(path.join(pluginsDir, name), body);
  }
  return { root, pluginsDir };
}

describe("discoverAppPlugins", () => {
  it("lists *.client and *.server files alphabetically and ignores unsuffixed", () => {
    const { root, pluginsDir } = makeFixture({
      "z.client.ts": "export default {}",
      "a.server.ts": "export default {}",
      "m.client.tsx": "export default {}",
      "skip.ts": "export {}",
      "runtime.ts": "export {}",
    });

    const found = discoverAppPlugins(pluginsDir, root);

    expect(found.client).toEqual(["./src/plugins/m.client.tsx", "./src/plugins/z.client.ts"]);
    expect(found.server).toEqual(["./src/plugins/a.server.ts"]);
  });

  it("returns empty lists when the plugins dir is missing", () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "app-plugins-missing-"));
    tempDirs.push(root);
    expect(discoverAppPlugins(path.join(root, "nope"), root)).toEqual({
      client: [],
      server: [],
    });
  });
});
