pub mod error;
mod server;

pub use error::{AssetServiceError, Result};
pub use server::{AssetService, ProtoAssetService, ProtoAssetServiceServer};
