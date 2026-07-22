// Faro persists sessions in sessionStorage. Resumed sessions keep a stored
// `isSampled` flag; legacy entries without the field become `false` via
// `stored.isSampled || false`. Faro's tracing sampler then returns NOT_RECORD
// while fetch instrumentation still injects traceparent — Tempo orphans.
// Patch the stored session so self-hosted always records.

export const FARO_SESSION_STORAGE_KEY = "com.grafana.faro.session";

type StoredFaroSession = {
  isSampled?: boolean;
  sessionMeta?: {
    attributes?: Record<string, string>;
    [key: string]: unknown;
  };
  [key: string]: unknown;
};

/** Ensure a resumed Faro session is marked sampled before `initializeFaro`. */
export function ensureFaroSessionSampled(
  storage: Pick<Storage, "getItem" | "setItem"> | null | undefined = globalThis.sessionStorage,
): void {
  if (!storage) return;
  let raw: string | null;
  try {
    raw = storage.getItem(FARO_SESSION_STORAGE_KEY);
  } catch {
    return;
  }
  if (!raw) return;

  let parsed: StoredFaroSession;
  try {
    parsed = JSON.parse(raw) as StoredFaroSession;
  } catch {
    return;
  }

  const attrs = parsed.sessionMeta?.attributes;
  if (parsed.isSampled === true && attrs?.isSampled === "true") return;

  parsed.isSampled = true;
  parsed.sessionMeta = {
    ...parsed.sessionMeta,
    attributes: {
      ...attrs,
      isSampled: "true",
    },
  };

  try {
    storage.setItem(FARO_SESSION_STORAGE_KEY, JSON.stringify(parsed));
  } catch {
    // Quota / private mode — Faro will create a fresh session.
  }
}
