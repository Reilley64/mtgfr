/**
 * Production client (browser) build options shared with vite.config.
 *
 * Source maps are intentionally public (`true`, not `"hidden"`) so Chrome
 * DevTools and Faro can resolve the large first-party bundle without a
 * separate upload pipeline. Trade-off: original TS sources are fetchable.
 */
export const clientBuildSourcemap = true as const;
