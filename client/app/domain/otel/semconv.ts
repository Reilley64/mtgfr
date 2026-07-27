/** OTel Semantic Conventions 1.37.0 — allowlisted keys only (observability-ops design). */

export const HTTP_REQUEST_METHOD = "http.request.method";
export const HTTP_RESPONSE_STATUS_CODE = "http.response.status_code";
export const HTTP_ROUTE = "http.route";
export const RPC_SYSTEM = "rpc.system";
export const RPC_SERVICE = "rpc.service";
export const RPC_METHOD = "rpc.method";
export const RPC_GRPC_STATUS_CODE = "rpc.grpc.status_code";
export const DB_SYSTEM = "db.system";
export const DB_OPERATION_NAME = "db.operation.name";
export const DB_NAMESPACE = "db.namespace";
export const EXCEPTION_TYPE = "exception.type";

export const MTGFR_TABLE_ID = "mtgfr.table.id";
export const MTGFR_INTENT_KIND = "mtgfr.intent.kind";
export const MTGFR_INTENT_ACCEPTED = "mtgfr.intent.accepted";
export const MTGFR_USER_ID = "mtgfr.user.id";

export const FORBIDDEN_ATTR_KEYS = new Set<string>([
  "db.query.text",
  "db.statement",
  "mtgfr.intent.payload",
  "http.request.body",
  "http.response.body",
  "Authorization",
  "authorization",
  "cookie",
  "Cookie",
  // legacy free-form keys we migrate away from
  "table_id",
  "intent.kind",
  "accepted",
  "user_id",
  "http.method",
  "rpc.path",
]);

export function httpServerAttrs(input: {
  method: string;
  route: string;
  statusCode?: number;
}): Record<string, string | number> {
  const attrs: Record<string, string | number> = {
    [HTTP_REQUEST_METHOD]: input.method,
    [HTTP_ROUTE]: input.route,
  };
  if (input.statusCode !== undefined) {
    attrs[HTTP_RESPONSE_STATUS_CODE] = input.statusCode;
  }
  return attrs;
}

export function rpcAttrs(input: {
  service: string;
  method: string;
  statusCode?: number;
}): Record<string, string | number> {
  const attrs: Record<string, string | number> = {
    [RPC_SYSTEM]: "grpc",
    [RPC_SERVICE]: input.service,
    [RPC_METHOD]: input.method,
  };
  if (input.statusCode !== undefined) {
    attrs[RPC_GRPC_STATUS_CODE] = input.statusCode;
  }
  return attrs;
}

export function dbAttrs(input: {
  operation: string;
  namespace: string;
}): Record<string, string> {
  return {
    [DB_SYSTEM]: "postgresql",
    [DB_OPERATION_NAME]: input.operation,
    [DB_NAMESPACE]: input.namespace,
  };
}

export function assertNoForbiddenKeys(attrs: Record<string, unknown>): void {
  for (const key of Object.keys(attrs)) {
    if (FORBIDDEN_ATTR_KEYS.has(key)) {
      throw new Error(`forbidden otel attribute key: ${key}`);
    }
    if (key.startsWith("db.query.")) {
      throw new Error(`forbidden otel attribute key: ${key}`);
    }
  }
}
