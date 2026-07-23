import { Scene } from "foldkit/test";
import { test } from "vitest";
import { init, update } from "../../main-exports";
import { LoginRoute } from "../../routes";
import { view } from "../../view";

test("hides the API badge until the version is fetched", () => {
  const [model] = init();
  Scene.scene(
    { update, view },
    Scene.with({ ...model, route: LoginRoute(), apiVersion: null }),
    Scene.expect(Scene.selector('[data-testid="app-version"]')).not.toExist(),
  );
});

test("renders the fetched API badge on auth", () => {
  const [model] = init();
  Scene.scene(
    { update, view },
    Scene.with({ ...model, route: LoginRoute(), apiVersion: "1.2.3" }),
    Scene.expect(Scene.selector('[data-testid="app-version"]')).toExist(),
    Scene.expect(Scene.text("API 1.2.3")).toExist(),
  );
});

test("auth panel exposes data-testid", () => {
  const [model] = init();
  Scene.scene(
    { update, view },
    Scene.with({ ...model, route: LoginRoute() }),
    Scene.expect(Scene.selector('[data-testid="auth-panel"]')).toExist(),
  );
});
