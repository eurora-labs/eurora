/// REST path prefixes that bypass authorization (public/unauthenticated routes).
pub const REST_BYPASS_PREFIXES: &[&str] = &["/releases/", "/extensions/"];

/// REST paths that bypass authorization via exact match.
pub const REST_BYPASS_EXACT: &[&str] = &["/payment/webhook"];

/// gRPC fully-qualified service names that bypass authorization.
pub const GRPC_BYPASS_SERVICES: &[&str] = &[
    "auth_service.ProtoAuthService",
    "grpc.health.v1.Health",
    "local_config_service.ProtoLocalConfigService",
];

/// Returns `true` if the given REST path should skip authorization.
pub fn is_rest_bypass(path: &str) -> bool {
    REST_BYPASS_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
        || REST_BYPASS_EXACT.contains(&path)
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
