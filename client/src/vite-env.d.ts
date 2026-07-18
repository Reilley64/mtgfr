/// <reference types="vinxi/types/client" />

declare module "virtual:app-plugins-client" {}

interface ImportMetaEnv {
  readonly VITE_APP_VERSION?: string;
  readonly VITE_GIT_COMMIT?: string;
  readonly VITE_CARD_CDN?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
