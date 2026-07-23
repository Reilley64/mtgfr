import type { html as createHtml, Html } from "foldkit/html";
import { appVersionClass } from "./surfaces";

/** Fixed bottom-left API badge — hidden until `version` is known (Solid AppVersion parity). */
export function appVersionBadge<M>(h: ReturnType<typeof createHtml<M>>, version: string | null): Html | null {
  if (version == null) return null;
  return h.div([h.DataAttribute("testid", "app-version"), h.Class(appVersionClass())], [`API ${version}`]);
}
