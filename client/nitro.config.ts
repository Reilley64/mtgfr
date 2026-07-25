import { defineConfig } from "nitro";

export default defineConfig({
  // Pin Bun so production `.output` is the Bun.serve build, not Node auto-detect.
  preset: "bun",
  renderer: {
    static: true,
    template: "./index.html",
  },
  serverDir: "./server",
});
