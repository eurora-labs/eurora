use be_analytics::{Event, capture_async};

pub fn track_activity_session_inserted(has_icon: bool, has_ended_at: bool, identity_key: &str) {
    let mut event = Event::new_anon("activity_session_inserted");
    event.insert_prop("has_icon", has_icon).ok();
    event.insert_prop("has_ended_at", has_ended_at).ok();
    event.insert_prop("identity_key", identity_key).ok();
    capture_async(event);
}

pub fn track_activity_insert_failed(error_kind: &str) {
    let mut event = Event::new_anon("activity_session_insert_failed");
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

pub fn track_activities_list_failed(error_kind: &str) {
    let mut event = Event::new_anon("activities_list_failed");
    event.insert_prop("error_kind", error_kind).ok();
    capture_async(event);
}

pub fn track_activity_session_updated(set_ended_at: bool, set_window_title: bool, set_url: bool) {
    let mut event = Event::new_anon("activity_session_updated");
    event.insert_prop("set_ended_at", set_ended_at).ok();
    event.insert_prop("set_window_title", set_window_title).ok();
    event.insert_prop("set_url", set_url).ok();
    capture_async(event);
}

pub fn track_activity_session_update_failed(error_kind: &str) {
    let mut event = Event::new_anon("activity_session_update_failed");
    event.insert_prop("error_kind", error_kind).ok();
    capture_async(event);
}
