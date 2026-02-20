use posthog_rs::Event;

fn capture_async(event: Event) {
    tokio::spawn(async move {
        if let Err(e) = posthog_rs::capture(event).await {
            tracing::error!("Failed to capture posthog event: {}", e);
        }
    });
}

pub fn track_update_check(
    channel: &str,
    target: &str,
    arch: &str,
    current_version: &str,
    bundle_type: Option<&str>,
    update_available: bool,
    latest_version: Option<&str>,
) {
    let mut event = Event::new_anon("update_check");
    event.insert_prop("channel", channel).ok();
    event.insert_prop("target", target).ok();
    event.insert_prop("arch", arch).ok();
    event.insert_prop("current_version", current_version).ok();
    if let Some(bt) = bundle_type {
        event.insert_prop("bundle_type", bt).ok();
    }
    event.insert_prop("update_available", update_available).ok();
    if let Some(v) = latest_version {
        event.insert_prop("latest_version", v).ok();
    }
    capture_async(event);
}

pub fn track_update_check_failed(
    channel: &str,
    target_arch: &str,
    current_version: &str,
    error_kind: &str,
) {
    let mut event = Event::new_anon("update_check_failed");
    event.insert_prop("channel", channel).ok();
    event.insert_prop("target_arch", target_arch).ok();
    event.insert_prop("current_version", current_version).ok();
    event.insert_prop("error_kind", error_kind).ok();
    capture_async(event);
}

pub fn track_download_redirect(channel: &str, target: &str, arch: &str, bundle_type: Option<&str>) {
    let mut event = Event::new_anon("download_redirect");
    event.insert_prop("channel", channel).ok();
    event.insert_prop("target", target).ok();
    event.insert_prop("arch", arch).ok();
    if let Some(bt) = bundle_type {
        event.insert_prop("bundle_type", bt).ok();
    }
    capture_async(event);
}

pub fn track_download_failed(
    channel: &str,
    target_arch: &str,
    bundle_type: Option<&str>,
    error_kind: &str,
) {
    let mut event = Event::new_anon("download_failed");
    event.insert_prop("channel", channel).ok();
    event.insert_prop("target_arch", target_arch).ok();
    if let Some(bt) = bundle_type {
        event.insert_prop("bundle_type", bt).ok();
    }
    event.insert_prop("error_kind", error_kind).ok();
    capture_async(event);
}

pub fn track_release_info_request(channel: &str, version: Option<&str>, platform_count: usize) {
    let mut event = Event::new_anon("release_info_request");
    event.insert_prop("channel", channel).ok();
    if let Some(v) = version {
        event.insert_prop("version", v).ok();
    }
    event.insert_prop("platform_count", platform_count).ok();
    capture_async(event);
}

pub fn track_extension_check(channel: &str, browsers_available: &[String]) {
    let mut event = Event::new_anon("extension_check");
    event.insert_prop("channel", channel).ok();
    event
        .insert_prop("browsers_available", browsers_available.join(","))
        .ok();
    capture_async(event);
}

pub fn track_extension_check_failed(channel: &str, error_kind: &str) {
    let mut event = Event::new_anon("extension_check_failed");
    event.insert_prop("channel", channel).ok();
    event.insert_prop("error_kind", error_kind).ok();
    capture_async(event);
}
