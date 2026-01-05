// Include the generated proto code
pub mod proto {
    tonic::include_proto!("activity_service");
}

mod assets;
mod types;

pub use types::Activity;
