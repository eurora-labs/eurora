//! Asset implementations for different activity types

pub mod article;
pub mod default;
pub mod twitter;
pub mod youtube;

pub use article::ArticleAsset;
pub use default::DefaultAsset;
pub use twitter::TwitterAsset;
pub use youtube::YoutubeAsset;
