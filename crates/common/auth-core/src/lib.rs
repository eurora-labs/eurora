//! Shared wire types for the Eurora auth HTTP service.
//!
//! This crate is the single source of truth for the JSON contract between
//! `be-auth-service` (Axum) and the desktop / web HTTP clients, and is also
//! the input to the TypeScript bindings emitted by the workspace-level
//! `euro-api-codegen` orchestrator (`pnpm specta:backend`).
//!
//! Types are pure data with `serde` derives; the optional `specta` feature
//! adds `specta::Type` so the same definitions can be re-exported as TS.
//! No HTTP, database, or gRPC dependencies live here on purpose — pulling
//! this crate into a leaf binary must not drag in transport plumbing.

pub mod claims;
pub mod error_kinds;
pub mod provider;
pub mod requests;
pub mod responses;

pub use claims::{Claims, Role};
pub use provider::Provider;
pub use requests::{
    AppleIdTokenLoginRequest, AppleNativeUser, AssociateLoginTokenRequest, CheckEmailRequest,
    GoogleIdTokenLoginRequest, LoginByLoginTokenRequest, LoginRequest,
    MobileThirdPartyAuthUrlRequest, RegisterRequest, ThirdPartyAuthUrlRequest, VerifyEmailRequest,
};
pub use responses::{
    AuthErrorResponse, AuthSuccessResponse, CheckEmailResponse, CheckEmailStatus,
    ThirdPartyAuthUrlResponse, TokenResponse, UserInfo, UserResponse,
};

/// Build a [`specta::Types`] containing every auth wire type
/// the desktop / mobile app needs. Used by the codegen binary to emit
/// `auth.ts`.
#[cfg(feature = "specta")]
pub fn type_collection() -> specta::Types {
    specta::Types::default()
        .register::<Claims>()
        .register::<Role>()
        .register::<Provider>()
        .register::<LoginRequest>()
        .register::<RegisterRequest>()
        .register::<ThirdPartyAuthUrlRequest>()
        .register::<ThirdPartyAuthUrlResponse>()
        .register::<MobileThirdPartyAuthUrlRequest>()
        .register::<GoogleIdTokenLoginRequest>()
        .register::<AppleIdTokenLoginRequest>()
        .register::<AppleNativeUser>()
        .register::<LoginByLoginTokenRequest>()
        .register::<AssociateLoginTokenRequest>()
        .register::<CheckEmailRequest>()
        .register::<CheckEmailResponse>()
        .register::<CheckEmailStatus>()
        .register::<VerifyEmailRequest>()
        .register::<TokenResponse>()
        .register::<UserInfo>()
        .register::<UserResponse>()
        .register::<AuthSuccessResponse>()
        .register::<AuthErrorResponse>()
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "specta")]
    #[test]
    fn type_collection_contains_all_wire_types() {
        let types = super::type_collection();
        let names: Vec<String> = types
            .into_unsorted_iter()
            .map(|ndt| ndt.name.to_string())
            .collect();
        for expected in [
            "Claims",
            "Role",
            "Provider",
            "LoginRequest",
            "RegisterRequest",
            "ThirdPartyAuthUrlRequest",
            "ThirdPartyAuthUrlResponse",
            "MobileThirdPartyAuthUrlRequest",
            "GoogleIdTokenLoginRequest",
            "AppleIdTokenLoginRequest",
            "AppleNativeUser",
            "LoginByLoginTokenRequest",
            "AssociateLoginTokenRequest",
            "CheckEmailRequest",
            "CheckEmailResponse",
            "CheckEmailStatus",
            "VerifyEmailRequest",
            "TokenResponse",
            "UserInfo",
            "UserResponse",
            "AuthSuccessResponse",
            "AuthErrorResponse",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing {expected} from collection: {names:?}"
            );
        }
    }
}
