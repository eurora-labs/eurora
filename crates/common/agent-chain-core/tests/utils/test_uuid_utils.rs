use std::time::{SystemTime, UNIX_EPOCH};

use agent_chain_core::utils::uuid::uuid7;
use uuid::Uuid;

fn uuid_v7_ms(uuid_obj: &Uuid) -> u64 {
    let s = uuid_obj.to_string().replace("-", "");
    u64::from_str_radix(&s[..12], 16).expect("Failed to parse UUID timestamp")
}

#[test]
fn test_uuid7() {
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos() as u64;
    let ms = ns / 1_000_000;
    let out1 = uuid7(Some(ns));

    let out1_ms = uuid_v7_ms(&out1);
    assert_eq!(out1_ms, ms);
}

#[test]
fn test_monotonicity() {
    let mut last = String::new();
    for n in 0..100_000 {
        let i = uuid7(None).to_string();
        if n > 0 && i <= last {
            panic!("UUIDs are not monotonic: {} versus {}", last, i);
        }
        last = i;
    }
}
