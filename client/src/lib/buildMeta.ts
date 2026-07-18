/** Release identity for OTEL / Faro — baked at image build, overridable via env. */

export function appVersion(
  env: NodeJS.ProcessEnv = process.env,
  viteVersion: string | undefined = import.meta.env.VITE_APP_VERSION,
): string {
  const fromEnv = env.APP_VERSION?.trim();
  if (fromEnv) return fromEnv;
  const fromVite = viteVersion?.trim();
  if (fromVite) return fromVite;
  return "dev";
}

export function gitCommit(
  env: NodeJS.ProcessEnv = process.env,
  viteCommit: string | undefined = import.meta.env.VITE_GIT_COMMIT,
): string {
  const fromEnv = env.GIT_COMMIT?.trim();
  if (fromEnv) return fromEnv;
  const fromVite = viteCommit?.trim();
  if (fromVite) return fromVite;
  return "unknown";
}
