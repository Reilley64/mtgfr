import * as Dialog from "@foldkit/ui/dialog";
import * as Command from "foldkit/command";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { test } from "vitest";
import { confirmDialog } from "./confirmDialog";

// A stand-in owner: the smallest thing that holds a Dialog.Model and hears the confirm message.
type HostModel = {
  dialog: Dialog.Model;
  danger: boolean;
  body: string | undefined;
  deleted: boolean;
};

type HostMessage = { _tag: "ConfirmedDelete" } | { _tag: "GotDialogMessage"; message: Dialog.Message };

const h = html<HostMessage>();

function hostModel(overrides: Partial<HostModel> = {}): HostModel {
  return {
    dialog: Dialog.init({ id: "delete-deck", isOpen: true }),
    danger: false,
    body: "This cannot be undone.",
    deleted: false,
    ...overrides,
  };
}

const toDialogMessage = (message: Dialog.Message): HostMessage => ({ _tag: "GotDialogMessage", message });

function update(model: HostModel, message: HostMessage): readonly [HostModel, ReadonlyArray<unknown>] {
  if (message._tag === "ConfirmedDelete") return [{ ...model, deleted: true }, []];

  const [dialog, commands] = Dialog.update(model.dialog, message.message);
  return [{ ...model, dialog }, Command.mapMessages(commands, toDialogMessage)];
}

const view = (model: HostModel) =>
  h.div(
    [],
    [
      confirmDialog(h, {
        model: model.dialog,
        toDialogMessage,
        title: "Delete deck?",
        body: model.body,
        confirmLabel: "Delete",
        danger: model.danger,
        onConfirm: { _tag: "ConfirmedDelete" },
        testId: "delete-dialog",
      }),
      model.deleted ? h.div([h.DataAttribute("testid", "deleted")], ["gone"]) : null,
    ],
  );

const program = { update, view } as never;

test("a closed prompt keeps its dialog element but shows nothing", () => {
  Scene.scene(
    program,
    Scene.with(hostModel({ dialog: Dialog.init({ id: "delete-deck" }) })),
    Scene.expect(Scene.testId("delete-dialog")).toExist(),
    Scene.expect(Scene.testId("confirm-title")).toBeAbsent(),
    Scene.expect(Scene.testId("confirm-ok")).toBeAbsent(),
  );
});

// Regression: a closed `<dialog>` is hidden only by the UA rule `dialog:not([open]) { display: none }`.
// `flex` overrode it, so every closed modal stayed a full-viewport `pointer-events-auto` layer and ate
// clicks on the page behind it — the deck list's "New deck" tile stopped responding entirely.
test("a closed prompt does not lay itself out, so it cannot cover the page", () => {
  Scene.scene(
    program,
    Scene.with(hostModel({ dialog: Dialog.init({ id: "delete-deck" }) })),
    Scene.expect(Scene.testId("delete-dialog")).not.toHaveClass("flex"),
  );
});

test("an open prompt centres itself over the page", () => {
  Scene.scene(program, Scene.with(hostModel()), Scene.expect(Scene.testId("delete-dialog")).toHaveClass("flex"));
});

test("an open prompt shows its question, its detail, and both choices", () => {
  Scene.scene(
    program,
    Scene.with(hostModel()),
    Scene.expect(Scene.text("Delete deck?")).toExist(),
    Scene.expect(Scene.text("This cannot be undone.")).toExist(),
    Scene.expect(Scene.testId("confirm-ok")).toHaveText("Delete"),
    Scene.expect(Scene.testId("confirm-cancel")).toHaveText("Cancel"),
  );
});

test("a prompt with no detail text renders no description", () => {
  Scene.scene(
    program,
    Scene.with(hostModel({ body: undefined })),
    Scene.expect(Scene.text("Delete deck?")).toExist(),
    Scene.expect(Scene.selector(`#${Dialog.descriptionId(Dialog.init({ id: "delete-deck" }))}`)).toBeAbsent(),
  );
});

test("the prompt is named by its own question", () => {
  const titleId = Dialog.titleId(Dialog.init({ id: "delete-deck" }));

  Scene.scene(
    program,
    Scene.with(hostModel()),
    Scene.expect(Scene.testId("delete-dialog")).toHaveAttr("aria-labelledby", titleId),
    Scene.expect(Scene.selector(`#${titleId}`)).toHaveText("Delete deck?"),
  );
});

// Dialog focuses whichever element carries this marker when it opens. `@foldkit/ui/dialog`'s
// public entry does not re-export `initialFocusMarkerAttribute`, so the name is spelled out here.
const INITIAL_FOCUS_ATTR = "data-foldkit-dialog-initial-focus";

test("focus lands on Cancel, so a destructive confirm is never one Enter away", () => {
  Scene.scene(
    program,
    Scene.with(hostModel()),
    Scene.expect(Scene.testId("confirm-cancel")).toHaveAttr(INITIAL_FOCUS_ATTR, ""),
    Scene.expect(Scene.testId("confirm-ok")).not.toHaveAttr(INITIAL_FOCUS_ATTR, ""),
  );
});

test("Cancel dismisses the prompt and deletes nothing", () => {
  Scene.scene(
    program,
    Scene.with(hostModel()),
    Scene.click(Scene.testId("confirm-cancel")),
    Scene.Command.resolve(Dialog.CloseDialog, Dialog.CompletedCloseDialog()),
    Scene.expect(Scene.testId("confirm-title")).toBeAbsent(),
    Scene.expect(Scene.testId("deleted")).toBeAbsent(),
  );
});

test("Confirm tells the owner to go ahead", () => {
  Scene.scene(
    program,
    Scene.with(hostModel()),
    Scene.click(Scene.testId("confirm-ok")),
    Scene.expect(Scene.testId("deleted")).toExist(),
  );
});

test("a destructive prompt paints Confirm in burn-red, an ordinary one in llanowar", () => {
  Scene.scene(
    program,
    Scene.with(hostModel({ danger: true })),
    Scene.expect(Scene.testId("confirm-ok")).toHaveClass("text-burn-red"),
  );

  Scene.scene(program, Scene.with(hostModel()), Scene.expect(Scene.testId("confirm-ok")).toHaveClass("bg-llanowar"));
});

test("the page behind an open prompt is dimmed and dismisses it when clicked", () => {
  Scene.scene(
    program,
    Scene.with(hostModel()),
    Scene.expect(Scene.testId("confirm-backdrop")).toHaveClass("bg-black/60"),
    Scene.click(Scene.testId("confirm-backdrop")),
    Scene.Command.resolve(Dialog.CloseDialog, Dialog.CompletedCloseDialog()),
    Scene.expect(Scene.testId("confirm-title")).toBeAbsent(),
  );
});
