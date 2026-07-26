/** Release identity for OTEL / Faro — baked at image build, overridable via env. */

type EnvBag = Record<string, string | undefined>;

/** Node `process.env` when present; empty in the browser (Vite has no `process` global). */
function processEnv(): EnvBag {
  if (typeof process === "undefined") return {};
  return (process.env ?? {}) as EnvBag;
}

export function appVersion(
  env: EnvBag = processEnv(),
  viteVersion: string | undefined = import.meta.env.VITE_APP_VERSION,
): string {
  const fromEnv = env.APP_VERSION?.trim();
  if (fromEnv) return fromEnv;
  const fromVite = viteVersion?.trim();
  if (fromVite) return fromVite;
  return "dev";
}

export function gitCommit(
  env: EnvBag = processEnv(),
  viteCommit: string | undefined = import.meta.env.VITE_GIT_COMMIT,
): string {
  const fromEnv = env.GIT_COMMIT?.trim();
  if (fromEnv) return fromEnv;
  const fromVite = viteCommit?.trim();
  if (fromVite) return fromVite;
  return "unknown";
}
