//! Print N v7 UUIDs to stdout, one per line. Used to mint stable IDs for
//! the dev seed CSVs.
//!
//! Usage:
//!   cargo run -p be-auth-service --example seed_uuids -- 10

fn main() {
    let count: usize = std::env::args()
        .nth(1)
        .as_deref()
        .unwrap_or("1")
        .parse()
        .expect("count must be a positive integer");
    for _ in 0..count {
        println!("{}", uuid::Uuid::now_v7());
    }
}
