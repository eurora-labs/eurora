//! Password / email validation and hashing helpers.

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use email_address::EmailAddress;

use crate::error::{AuthError, AuthResult};

pub(crate) const MIN_PASSWORD_LENGTH: usize = 12;
pub(crate) const MAX_PASSWORD_LENGTH: usize = 128;

pub(crate) fn validate_email(email: &str) -> AuthResult<()> {
    if EmailAddress::is_valid(email) {
        Ok(())
    } else {
        Err(AuthError::InvalidInput(
            "Invalid email address format".into(),
        ))
    }
}

pub(crate) fn validate_password(password: &str) -> AuthResult<()> {
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(AuthError::InvalidInput(format!(
            "Password must be at least {MIN_PASSWORD_LENGTH} characters"
        )));
    }
    if password.len() > MAX_PASSWORD_LENGTH {
        return Err(AuthError::InvalidInput(format!(
            "Password must be at most {MAX_PASSWORD_LENGTH} characters"
        )));
    }
    Ok(())
}

pub(crate) fn hash_password(password: &str) -> AuthResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AuthError::PasswordHash(e.to_string()))?;
    Ok(hash.to_string())
}

pub(crate) fn verify_password(password: &str, stored_hash: &str) -> AuthResult<()> {
    let parsed_hash = PasswordHash::new(stored_hash)
        .map_err(|_| AuthError::Internal("Invalid stored password hash".into()))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AuthError::InvalidCredentials)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_password_rejects_short() {
        let err = validate_password("short").unwrap_err();
        assert!(matches!(err, AuthError::InvalidInput(_)));
    }

    #[test]
    fn validate_password_rejects_too_long() {
        let pw = "a".repeat(MAX_PASSWORD_LENGTH + 1);
        let err = validate_password(&pw).unwrap_err();
        assert!(matches!(err, AuthError::InvalidInput(_)));
    }

    #[test]
    fn validate_password_accepts_minimum_length() {
        let pw = "a".repeat(MIN_PASSWORD_LENGTH);
        validate_password(&pw).expect("min length must pass");
    }

    #[test]
    fn validate_email_accepts_valid() {
        validate_email("user@example.com").expect("valid email must pass");
    }

    #[test]
    fn validate_email_rejects_garbage() {
        assert!(validate_email("not-an-email").is_err());
        assert!(validate_email("a@.c").is_err());
        assert!(validate_email("@b.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("user with space@example.com").is_err());
    }

    #[test]
    fn hash_password_roundtrip() {
        let hash = hash_password("correct horse battery staple").unwrap();
        verify_password("correct horse battery staple", &hash).expect("matching password verifies");
        assert!(verify_password("wrong password here!", &hash).is_err());
    }
}
