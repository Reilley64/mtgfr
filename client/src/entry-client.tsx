// @refresh reload

import "virtual:app-plugins-client";
import { mount, StartClient } from "@solidjs/start/client";

mount(() => <StartClient />, document.getElementById("app")!);
