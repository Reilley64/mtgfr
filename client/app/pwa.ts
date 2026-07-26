import { registerSW } from "virtual:pwa-register";

export function registerPwa(): void {
  if (!import.meta.env.PROD) {
    return;
  }

  if (!("serviceWorker" in navigator)) {
    return;
  }

  registerSW({ immediate: true });
}
