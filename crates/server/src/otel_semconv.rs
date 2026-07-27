//! OpenTelemetry Semantic Conventions 1.37.0 helpers (observability-ops design).

pub const RPC_SYSTEM: &str = "rpc.system";
pub const RPC_SERVICE: &str = "rpc.service";
pub const RPC_METHOD: &str = "rpc.method";
pub const RPC_GRPC_STATUS_CODE: &str = "rpc.grpc.status_code";

pub const MTGFR_TABLE_ID: &str = "mtgfr.table.id";
pub const MTGFR_INTENT_KIND: &str = "mtgfr.intent.kind";
pub const MTGFR_INTENT_ACCEPTED: &str = "mtgfr.intent.accepted";
pub const MTGFR_USER_ID: &str = "mtgfr.user.id";

pub const FORBIDDEN_ATTR_KEYS: &[&str] = &[
    "db.query.text",
    "db.statement",
    "mtgfr.intent.payload",
    "table_id",
    "intent.kind",
    "accepted",
    "user_id",
];

/// `/mtgfr.v1.Game/SubmitIntent` → (`mtgfr.v1.Game`, `SubmitIntent`)
pub fn parse_grpc_path(path: &str) -> Option<(&str, &str)> {
    let trimmed = path.trim().trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let (service, method) = trimmed.split_once('/')?;
    if service.is_empty() || method.is_empty() || method.contains('/') {
        return None;
    }
    Some((service, method))
}

pub fn rpc_span_name(service: &str, method: &str) -> String {
    format!("{service}/{method}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_grpc_path_splits_service_and_method() {
        let (svc, method) =
            parse_grpc_path("/mtgfr.v1.Game/SubmitIntent").expect("path");
        assert_eq!(svc, "mtgfr.v1.Game");
        assert_eq!(method, "SubmitIntent");
        assert_eq!(rpc_span_name(svc, method), "mtgfr.v1.Game/SubmitIntent");
    }

    #[test]
    fn parse_grpc_path_rejects_garbage() {
        assert!(parse_grpc_path("").is_none());
        assert!(parse_grpc_path("/only-one").is_none());
    }

    #[test]
    fn forbidden_keys_include_query_text_and_legacy_names() {
        assert!(FORBIDDEN_ATTR_KEYS.contains(&"db.query.text"));
        assert!(FORBIDDEN_ATTR_KEYS.contains(&"intent.kind"));
        assert!(FORBIDDEN_ATTR_KEYS.contains(&"mtgfr.intent.payload"));
    }
}
