// @refresh reload

// Side-effect: boot auto-discovered client plugins (Faro/OTEL, …).
import "virtual:app-plugins-client";
// Belt-and-suspenders: keep Faro in the client graph even if the virtual
// module is tree-shaken (observed empty in prod 2.4.2 — no browser RUM).
import "~/plugins/otel.client";
import { mount, StartClient } from "@solidjs/start/client";

mount(() => <StartClient />, document.getElementById("app")!);
