import { definePlugin } from "nitro";
import { initOtel, shutdownOtel } from "../../app/domain/otel";

export default definePlugin((nitro) => {
  initOtel();
  nitro.hooks.hook("close", shutdownOtel);
});
