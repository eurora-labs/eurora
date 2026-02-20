use posthog_rs::Event;
use tracing::debug;

fn capture_async(event: Event) {
    tokio::spawn(async move {
        if let Err(e) = posthog_rs::capture(event).await {
            debug!("Failed to capture posthog event: {}", e);
        }
    });
}

pub fn track_activity_inserted(has_icon: bool, has_ended_at: bool, process_name: &str) {
    let mut event = Event::new_anon("activity_inserted");
    event.insert_prop("has_icon", has_icon).ok();
    event.insert_prop("has_ended_at", has_ended_at).ok();
    event.insert_prop("process_name", process_name).ok();
    capture_async(event);
}

pub fn track_activity_insert_failed(error_kind: &str) {
    let mut event = Event::new_anon("activity_insert_failed");
    event.insert_prop("error_kind", error_kind).ok();
    capture_async(event);
}

pub fn track_activities_listed(limit: u32, offset: u32, result_count: usize) {
    let mut event = Event::new_anon("activities_listed");
    event.insert_prop("limit", limit).ok();
    event.insert_prop("offset", offset).ok();
    event.insert_prop("result_count", result_count).ok();
    capture_async(event);
}
