/// REST path prefixes that bypass authorization (public/unauthenticated routes).
pub(crate) const REST_BYPASS_PREFIXES: &[&str] = &["/releases/", "/extensions/"];

/// REST paths that bypass authorization via exact match.
pub(crate) const REST_BYPASS_EXACT: &[&str] = &["/payment/webhook"];

/// gRPC fully-qualified service names that bypass authorization.
pub(crate) const GRPC_BYPASS_SERVICES: &[&str] = &[
    "auth_service.ProtoAuthService",
    "grpc.health.v1.Health",
    "local_config_service.ProtoLocalConfigService",
];

/// Normalize a URL path by stripping the query string / fragment and resolving
/// `.` and `..` segments to prevent bypass via path traversal (e.g.
/// `/releases/../payment/checkout`).
fn normalize_path(path: &str) -> String {
    // Strip query string and fragment before normalizing segments.
    let path = path.split('?').next().unwrap_or(path);
    let path = path.split('#').next().unwrap_or(path);

    let mut segments: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                segments.pop();
            }
            s => segments.push(s),
        }
    }
    format!("/{}", segments.join("/"))
}

/// Returns `true` if the given REST path should skip authorization.
///
/// The path is normalized before checking to prevent traversal-based bypasses.
pub fn is_rest_bypass(path: &str) -> bool {
    let normalized = normalize_path(path);
    REST_BYPASS_PREFIXES
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
        || REST_BYPASS_EXACT.iter().any(|&exact| normalized == exact)
}

/// Returns `true` if the given gRPC service should skip authorization.
pub fn is_grpc_bypass(service: &str) -> bool {
    GRPC_BYPASS_SERVICES.contains(&service)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_bypass_prefix_match() {
        assert!(is_rest_bypass("/releases/nightly"));
        assert!(is_rest_bypass("/releases/stable/v1.0.0"));
        assert!(is_rest_bypass("/extensions/nightly"));
    }

    #[test]
    fn rest_bypass_exact_match() {
        assert!(is_rest_bypass("/payment/webhook"));
    }

    #[test]
    fn rest_bypass_rejects_non_matching() {
        assert!(!is_rest_bypass("/payment/checkout"));
        assert!(!is_rest_bypass("/api/users"));
        assert!(!is_rest_bypass("/releases")); // no trailing slash
    }

    #[test]
    fn rest_bypass_rejects_traversal_attacks() {
        // Path traversal must not trick the prefix check
        assert!(!is_rest_bypass("/releases/../payment/checkout"));
        assert!(!is_rest_bypass("/extensions/../admin/users"));
        assert!(!is_rest_bypass("/releases/../../etc/passwd"));
    }

    #[test]
    fn normalize_path_resolves_segments() {
        assert_eq!(normalize_path("/a/b/../c"), "/a/c");
        assert_eq!(normalize_path("/a/./b/c"), "/a/b/c");
        assert_eq!(normalize_path("/a/b/../../c"), "/c");
        assert_eq!(normalize_path("/../a"), "/a");
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn normalize_path_strips_query_and_fragment() {
        assert_eq!(normalize_path("/releases/foo?bar=1"), "/releases/foo");
        assert_eq!(normalize_path("/releases/foo#section"), "/releases/foo");
        assert_eq!(
            normalize_path("/releases/foo?bar=1#section"),
            "/releases/foo"
        );
        assert_eq!(normalize_path("/a/../b?q=1"), "/b");
    }

    #[test]
    fn grpc_bypass_known_services() {
        assert!(is_grpc_bypass("auth_service.ProtoAuthService"));
        assert!(is_grpc_bypass("grpc.health.v1.Health"));
        assert!(is_grpc_bypass(
            "local_config_service.ProtoLocalConfigService"
        ));
    }

    #[test]
    fn grpc_bypass_rejects_non_matching() {
        assert!(!is_grpc_bypass(
            "conversation_service.ProtoConversationService"
        ));
        assert!(!is_grpc_bypass("ProtoAuthService"));
        assert!(!is_grpc_bypass(""));
    }
}
