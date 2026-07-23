import { defineConfig } from "nitro";

export default defineConfig({
  renderer: {
    static: true,
    template: "./index.html",
  },
  serverDir: "./server",
});
