import { createResource, Show } from "solid-js";
import * as lobbyClient from "~/lib/lobbyClient";

/** API release tag — fixed bottom-left on every Felt (pre-game) page shell. */
export function AppVersion() {
  const [version] = createResource(() =>
    lobbyClient.apiVersion().then((v) => {
      const tag = v?.version?.trim();
      return tag ? tag : null;
    }),
  );

  return (
    <Show when={version()}>
      {(v) => (
        <div
          data-testid="app-version"
          class="pointer-events-none fixed bottom-md left-md z-10 text-label text-lichen/70"
        >
          API {v()}
        </div>
      )}
    </Show>
  );
}
