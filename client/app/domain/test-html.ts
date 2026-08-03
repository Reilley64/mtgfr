// ponytail: foldkit 0.134 removed `html<Message>()` — a view gets its builder from the render
// frame, and `inertHtml` is the only one obtainable outside one. Unit tests that call a view
// helper directly borrow it: the markup is identical, and a handler built here dispatches
// nowhere because nothing mounts the result. Scene tests stay the check for real dispatch.

import { type HtmlBuilder, inertHtml } from "foldkit/html";

export function testHtml<Message>(): HtmlBuilder<Message> {
  return inertHtml as unknown as HtmlBuilder<Message>;
}

// ponytail: a Submodel view's builder is typed by the frame's universe (its `ViewMessage`), which
// Scene narrows to whatever its `update` accepts. The phantom universe marker is invariant, so
// handing the scene's builder to the view needs a cast even though the Messages are a subset.
export function asBuilder<Message>(h: unknown): HtmlBuilder<Message> {
  return h as HtmlBuilder<Message>;
}
