import { definePlugin } from "nitro";
import { initOtel, shutdownOtel } from "../../lib/otel";

export default definePlugin((nitro) => {
  initOtel();
  nitro.hooks.hook("close", shutdownOtel);
});
