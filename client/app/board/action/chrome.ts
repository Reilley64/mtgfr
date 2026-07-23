// ActionChrome — the pre-submit HTML prompts (X, modal, cost picks) that live beside the engine
// pending_choice host.
//
// Foldkit port re-exports the html/prompts view — kept as a thin alias so downstream files can
// import from `board/action/chrome` matching the Solid file layout.

export { promptsView as chromeView } from "../html/prompts";
