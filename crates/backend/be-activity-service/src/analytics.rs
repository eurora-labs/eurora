use be_analytics::{Event, capture_async};

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

pub fn track_activities_list_failed(error_kind: &str) {
    let mut event = Event::new_anon("activities_list_failed");
    event.insert_prop("error_kind", error_kind).ok();
    capture_async(event);
}

pub fn track_activity_updated(set_ended_at: bool, set_window_title: bool, set_name: bool) {
    let mut event = Event::new_anon("activity_updated");
    event.insert_prop("set_ended_at", set_ended_at).ok();
    event.insert_prop("set_window_title", set_window_title).ok();
    event.insert_prop("set_name", set_name).ok();
    capture_async(event);
}

pub fn track_activity_update_failed(error_kind: &str) {
    let mut event = Event::new_anon("activity_update_failed");
    event.insert_prop("error_kind", error_kind).ok();
    capture_async(event);
}
