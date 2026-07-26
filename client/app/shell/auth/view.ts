import { Submodel } from "foldkit";
import { html } from "foldkit/html";
import type { AppChromeMeta } from "../../domain/ui/app-version";
import { buttonClass } from "../../domain/ui/buttonClass";
import { fieldClass, panelClass } from "../../domain/ui/surfaces";
import { shellFrame } from "../frame/shell-frame";
import {
  ChangedAuthEmail,
  ChangedAuthMode,
  ChangedAuthPassword,
  ChangedAuthUsername,
  type Message,
  SubmittedAuth,
} from "./messages";
import type { AuthSubmodel } from "./submodel";

const h = html<Message>();

export const view = Submodel.defineView<AuthSubmodel, Message, AppChromeMeta>((model, chrome) => {
  const isLogin = model.mode === "login";
  const modeToggle = isLogin ? ChangedAuthMode({ mode: "signup" }) : ChangedAuthMode({ mode: "login" });

  return shellFrame(h, {
    atmosphere: "auth",
    title: isLogin ? "Sign in" : "Create account",
    chrome,
    stage: h.div(
      [h.Class("flex justify-center py-xxl")],
      [
        h.section(
          [h.DataAttribute("testid", "auth-panel"), h.DataAttribute("ui", "panel"), h.Class(panelClass())],
          [
            h.form(
              [h.Class("contents"), h.DataAttribute("testid", "auth-form"), h.OnSubmit(SubmittedAuth())],
              [
                h.div(
                  [h.Class("flex flex-col gap-xs")],
                  [
                    h.div([h.Class("m-0 text-display tracking-[-0.02em]")], ["edh.reilley.dev"]),
                    h.h1([h.Class("m-0 text-lichen text-title")], [isLogin ? "Sign in" : "Create account"]),
                  ],
                ),
                h.label([h.Class("text-label text-lichen"), h.For("email")], ["Email"]),
                h.input([
                  h.Id("email"),
                  h.DataAttribute("testid", "auth-email"),
                  h.Type("email"),
                  h.Autocomplete("email"),
                  h.Value(model.email),
                  h.OnInput((email) => ChangedAuthEmail({ email })),
                  h.Class(fieldClass()),
                ]),
                isLogin ? null : h.label([h.Class("text-label text-lichen"), h.For("username")], ["Username"]),
                isLogin
                  ? null
                  : h.input([
                      h.Id("username"),
                      h.DataAttribute("testid", "auth-username"),
                      h.Type("text"),
                      h.Autocomplete("username"),
                      h.Value(model.username),
                      h.OnInput((username) => ChangedAuthUsername({ username })),
                      h.Class(fieldClass()),
                    ]),
                h.label([h.Class("text-label text-lichen"), h.For("password")], ["Password"]),
                h.input([
                  h.Id("password"),
                  h.DataAttribute("testid", "auth-password"),
                  h.Type("password"),
                  h.Autocomplete(isLogin ? "current-password" : "new-password"),
                  h.Value(model.password),
                  h.OnInput((password) => ChangedAuthPassword({ password })),
                  h.Class(fieldClass()),
                ]),
                h.button(
                  [
                    h.Type("submit"),
                    h.DataAttribute("testid", "auth-submit"),
                    h.Disabled(model.submitting),
                    h.Class(buttonClass("primary")),
                  ],
                  [isLogin ? "Sign in" : "Sign up"],
                ),
                model.error == null
                  ? null
                  : h.div(
                      [h.Role("alert"), h.Class("text-burn-red text-caption"), h.DataAttribute("testid", "auth-error")],
                      [model.error],
                    ),
                h.div(
                  [h.Class("text-label text-lichen")],
                  [
                    isLogin ? "No account? " : "Have an account? ",
                    h.button(
                      [
                        h.Type("button"),
                        h.DataAttribute("ui", "link"),
                        h.Class(buttonClass("link")),
                        h.DataAttribute("testid", "auth-toggle-mode"),
                        h.OnClick(modeToggle),
                      ],
                      [isLogin ? "Create one" : "Sign in"],
                    ),
                  ],
                ),
              ],
            ),
          ],
        ),
      ],
    ),
  });
});
