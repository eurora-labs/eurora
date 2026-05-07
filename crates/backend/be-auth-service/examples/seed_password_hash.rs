//! Print a fresh argon2 hash for the supplied password. Used to regenerate
//! the dev seed password hash baked into `scripts/seed/data/password_credentials.csv`.
//!
//! Usage:
//!   cargo run -p be-auth-service --example seed_password_hash -- <password>

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

fn main() {
    let password = std::env::args().nth(1).unwrap_or_else(|| "dev".to_string());
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash password");
    println!("{hash}");
}
