use uuid::Uuid;

const NANOS_PER_SECOND: u64 = 1_000_000_000;

fn to_timestamp_and_nanos(nanoseconds: u64) -> (u64, u32) {
    let seconds = nanoseconds / NANOS_PER_SECOND;
    let nanos = (nanoseconds % NANOS_PER_SECOND) as u32;
    (seconds, nanos)
}

pub fn uuid7(nanoseconds: Option<u64>) -> Uuid {
    use uuid::Timestamp;

    match nanoseconds {
        Some(nanos) => {
            let (secs, remaining_nanos) = to_timestamp_and_nanos(nanos);
            let ts = Timestamp::from_unix(uuid::NoContext, secs, remaining_nanos);
            Uuid::new_v7(ts)
        }
        None => Uuid::now_v7(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid7() {
        let id = uuid7(None);
        assert!(!id.is_nil());
    }

    #[test]
    fn test_uuid7_with_nanoseconds() {
        let nanos = 1_609_459_200_000_000_000_u64; // 2021-01-01 00:00:00 UTC
        let id = uuid7(Some(nanos));
        assert!(!id.is_nil());
    }

    #[test]
    fn test_to_timestamp_and_nanos() {
        let (secs, nanos) = to_timestamp_and_nanos(1_500_000_000);
        assert_eq!(secs, 1);
        assert_eq!(nanos, 500_000_000);
    }
}
