import { Story } from "foldkit";
import { expect, test } from "vitest";
import type { LobbyView } from "../../domain/lobby/types";
import { init, update } from "../../main-exports";
import { GotLobbyMessage } from "../../messages";
import { PregameTableRoute } from "../../routes";
import { ReceivedLobbyView } from "./messages";

const me = { id: 1, email: "alice@example.com", username: "alice" };

const startedLobby: LobbyView = {
  table_id: "ABC123",
  seats: [],
  you: 0,
  started: true,
  start_error: null,
  error: null,
};

test("started lobby view activates the board handoff", () => {
  const [model] = init();

  Story.story(
    update,
    Story.with({
      ...model,
      route: PregameTableRoute({ deckId: "0", table: "ABC123" }),
      sessionLoaded: true,
      session: { me, meGravatarHash: null },
      lobby: { ...model.lobby, tableId: "ABC123" },
    }),
    Story.message(GotLobbyMessage({ message: ReceivedLobbyView({ view: startedLobby }) })),
    Story.model((m) => {
      expect(m.lobby.started).toBe(true);
      expect(m.game?.active).toBe(true);
      expect(m.game?.tableId).toBe("ABC123");
      expect(m.game?.seq).toBe(0);
    }),
  );
});
