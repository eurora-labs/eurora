//! End-to-end checks on the token-generation primitive.
//!
//! These tests exercise the contract `JwtConfig::validate_*_token` and
//! `tokens::generate_jwt_pair` form together: we mint a pair, hand the
//! raw tokens to the validator, and assert the round-tripped claims
//! match what we asked for. The shared module below is intentionally
//! minimal — it does NOT pull in `be-auth-service::AuthService` so the
//! tests don't need a running database.

use std::collections::HashSet;

use auth_core::Role;
use be_auth_core::JwtConfig;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Validation};
use uuid::Uuid;

const TEST_SECRET: &[u8] = b"test-secret-do-not-use-in-production";

fn build_test_jwt_config() -> JwtConfig {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["eurora"]);
    validation.required_spec_claims.insert("aud".to_string());
    JwtConfig {
        access_token_encoding_key: EncodingKey::from_secret(TEST_SECRET),
        access_token_decoding_key: DecodingKey::from_secret(TEST_SECRET),
        refresh_token_encoding_key: EncodingKey::from_secret(TEST_SECRET),
        refresh_token_decoding_key: DecodingKey::from_secret(TEST_SECRET),
        access_token_expiry_hours: 1,
        refresh_token_expiry_days: 7,
        validation,
        approved_emails: HashSet::new(),
    }
}

mod helpers {
    //! Re-implementation of `tokens::generate_jwt_pair` for tests. The
    //! production helper is `pub(crate)`; copying the body here keeps
    //! the unit-test helper private without expanding the crate's
    //! public surface for testing-only access.
    use auth_core::{Claims, Role};
    use be_auth_core::JwtConfig;
    use chrono::{DateTime, Duration, Utc};
    use jsonwebtoken::{Algorithm, Header, encode};
    use uuid::Uuid;

    pub struct JwtPair {
        pub access_token: String,
        pub refresh_token: String,
        pub refresh_expires_at: DateTime<Utc>,
    }

    pub fn generate_jwt_pair(
        config: &JwtConfig,
        user_id: Uuid,
        email: &str,
        role: Role,
        email_verified: bool,
    ) -> JwtPair {
        let now = Utc::now();
        let access_exp = now + Duration::hours(config.access_token_expiry_hours);
        let refresh_exp = now + Duration::days(config.refresh_token_expiry_days);
        let sub = user_id.to_string();
        let aud = "eurora".to_string();

        let make_claims = |exp: DateTime<Utc>, token_type: &str| Claims {
            sub: sub.clone(),
            email: email.to_string(),
            display_name: None,
            exp: exp.timestamp(),
            iat: now.timestamp(),
            token_type: token_type.to_string(),
            role: role.clone(),
            aud: aud.clone(),
            email_verified,
            jti: Uuid::now_v7().to_string(),
        };

        let header = Header::new(Algorithm::HS256);
        let access_token = encode(
            &header,
            &make_claims(access_exp, "access"),
            &config.access_token_encoding_key,
        )
        .expect("encode access");
        let refresh_token = encode(
            &header,
            &make_claims(refresh_exp, "refresh"),
            &config.refresh_token_encoding_key,
        )
        .expect("encode refresh");

        JwtPair {
            access_token,
            refresh_token,
            refresh_expires_at: refresh_exp,
        }
    }
}

#[test]
fn pair_round_trips_through_validator() {
    let cfg = build_test_jwt_config();
    let user_id = Uuid::now_v7();
    let pair = helpers::generate_jwt_pair(&cfg, user_id, "u@example.com", Role::Free, true);

    let access_claims = cfg.validate_access_token(&pair.access_token).unwrap();
    assert_eq!(access_claims.sub, user_id.to_string());
    assert_eq!(access_claims.token_type, "access");
    assert_eq!(access_claims.aud, "eurora");
    assert!(access_claims.email_verified);

    let refresh_claims = cfg.validate_refresh_token(&pair.refresh_token).unwrap();
    assert_eq!(refresh_claims.sub, user_id.to_string());
    assert_eq!(refresh_claims.token_type, "refresh");
}

#[test]
fn access_token_rejected_by_refresh_validator() {
    let cfg = build_test_jwt_config();
    let pair = helpers::generate_jwt_pair(&cfg, Uuid::now_v7(), "u@example.com", Role::Free, true);
    assert!(cfg.validate_refresh_token(&pair.access_token).is_err());
}

#[test]
fn refresh_token_rejected_by_access_validator() {
    let cfg = build_test_jwt_config();
    let pair = helpers::generate_jwt_pair(&cfg, Uuid::now_v7(), "u@example.com", Role::Free, true);
    assert!(cfg.validate_access_token(&pair.refresh_token).is_err());
}

#[test]
fn jti_differs_between_pair_members() {
    let cfg = build_test_jwt_config();
    let pair = helpers::generate_jwt_pair(&cfg, Uuid::now_v7(), "u@example.com", Role::Tier1, true);
    let access = cfg.validate_access_token(&pair.access_token).unwrap();
    let refresh = cfg.validate_refresh_token(&pair.refresh_token).unwrap();
    assert_ne!(
        access.jti, refresh.jti,
        "access and refresh jti must differ"
    );
}

#[test]
fn refresh_expiry_outlives_access_expiry() {
    let cfg = build_test_jwt_config();
    let pair = helpers::generate_jwt_pair(&cfg, Uuid::now_v7(), "u@example.com", Role::Free, true);
    let access = cfg.validate_access_token(&pair.access_token).unwrap();
    let refresh = cfg.validate_refresh_token(&pair.refresh_token).unwrap();
    assert!(refresh.exp > access.exp);
    // refresh_expires_at returned to the caller must match the JWT
    assert_eq!(refresh.exp, pair.refresh_expires_at.timestamp());
}

#[test]
fn approved_emails_lookup_is_case_insensitive() {
    let mut cfg = build_test_jwt_config();
    cfg.approved_emails.insert("alice@example.com".into());
    assert!(cfg.is_approved_email("Alice@Example.com"));
    assert!(cfg.is_approved_email("ALICE@EXAMPLE.COM"));
    assert!(!cfg.is_approved_email("bob@example.com"));
}
