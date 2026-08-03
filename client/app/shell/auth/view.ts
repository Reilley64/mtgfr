import { Submodel } from "foldkit";
import type { AppChromeMeta } from "../../domain/ui/app-version";
import { button } from "../../domain/ui/button";
import { input } from "../../domain/ui/input";
import { alertClass, panelClass } from "../../domain/ui/surfaces";
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

export const view = Submodel.defineView<AuthSubmodel, Message, AppChromeMeta>((model, chrome, h) => {
  const isLogin = model.mode === "login";
  const modeToggle = isLogin ? ChangedAuthMode({ mode: "signup" }) : ChangedAuthMode({ mode: "login" });

  return shellFrame(h, {
    atmosphere: "auth",
    chrome,
    stage: h.div(
      [h.Class("flex flex-col items-center justify-center gap-md py-xxl text-center")],
      [
        h.div(
          [h.DataAttribute("testid", "auth-brand"), h.Class("m-0 font-display text-display tracking-display")],
          ["edh.reilley.dev"],
        ),
        h.h1([h.Class("m-0 font-display text-lichen text-title")], [isLogin ? "Sign in" : "Create account"]),
        h.section(
          [h.DataAttribute("testid", "auth-panel"), h.DataAttribute("ui", "panel"), h.Class(panelClass())],
          [
            h.form(
              [h.Class("contents"), h.DataAttribute("testid", "auth-form"), h.OnSubmit(SubmittedAuth())],
              [
                h.label([h.Class("text-label text-lichen"), h.For("email")], ["Email"]),
                input(h, {
                  id: "email",
                  testId: "auth-email",
                  type: "email",
                  value: model.email,
                  onInput: (email) => ChangedAuthEmail({ email }),
                  attrs: [h.Autocomplete("email")],
                }),
                isLogin ? null : h.label([h.Class("text-label text-lichen"), h.For("username")], ["Username"]),
                isLogin
                  ? null
                  : input(h, {
                      id: "username",
                      testId: "auth-username",
                      type: "text",
                      value: model.username,
                      onInput: (username) => ChangedAuthUsername({ username }),
                      attrs: [h.Autocomplete("username")],
                    }),
                h.label([h.Class("text-label text-lichen"), h.For("password")], ["Password"]),
                input(h, {
                  id: "password",
                  testId: "auth-password",
                  type: "password",
                  value: model.password,
                  onInput: (password) => ChangedAuthPassword({ password }),
                  attrs: [h.Autocomplete(isLogin ? "current-password" : "new-password")],
                }),
                button(h, { type: "submit", testId: "auth-submit", disabled: model.submitting, variant: "primary" }, [
                  isLogin ? "Sign in" : "Sign up",
                ]),
                model.error == null
                  ? null
                  : h.div(
                      [h.Role("alert"), h.Class(alertClass("text-burn-red")), h.DataAttribute("testid", "auth-error")],
                      [model.error],
                    ),
                h.div(
                  [h.Class("text-label text-lichen")],
                  [
                    isLogin ? "No account? " : "Have an account? ",
                    button(
                      h,
                      {
                        testId: "auth-toggle-mode",
                        onClick: modeToggle,
                        variant: "link",
                        attrs: [h.DataAttribute("ui", "link")],
                      },
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
