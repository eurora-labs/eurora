mod converters;
pub mod db;
pub mod error;
pub mod types;

pub use db::DatabaseManager;
pub use error::{DbError, DbResult};
pub use types::*;

pub fn year_month_key(now: &chrono::DateTime<chrono::Utc>) -> i32 {
    use chrono::Datelike;
    now.year() * 100 + now.month() as i32
}
