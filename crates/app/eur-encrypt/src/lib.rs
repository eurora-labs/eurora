mod error;

pub use error::{EncryptError, EncryptResult};

pub const USER_MAIN_KEY_HANDLE: &str = "USER_MAIN_KEY_HANDLE";

fn generate_new_key() -> EncryptResult<[u8; 32]> {
    todo!()
}

fn convert_key_to_base64(key: [u8; 32]) -> EncryptResult<String> {
    todo!()
}
