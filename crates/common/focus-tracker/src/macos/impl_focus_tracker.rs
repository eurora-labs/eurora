use crate::{FocusTrackerConfig, FocusTrackerResult, FocusedWindow, icon_cache::IconCache};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::utils;

#[inline]
fn should_stop(stop_signal: Option<&AtomicBool>) -> bool {
    stop_signal.is_some_and(|stop| stop.load(Ordering::Relaxed))
}

#[derive(Default)]
struct FocusState {
    process_id: u32,
    process_name: String,
    window_title: Option<String>,
}

impl FocusState {
    fn has_changed(&self, window: &FocusedWindow) -> bool {
        self.process_id != window.process_id
            || self.process_name != window.process_name
            || self.window_title.as_deref() != window.window_title.as_deref()
    }

    fn update_from(&mut self, window: &FocusedWindow) {
        self.process_id = window.process_id;
        self.process_name.clone_from(&window.process_name);
        self.window_title.clone_from(&window.window_title);
    }
}

pub(crate) async fn track_focus<F, Fut>(
    mut on_focus: F,
    stop_signal: Option<&AtomicBool>,
    config: &FocusTrackerConfig,
) -> FocusTrackerResult<()>
where
    F: FnMut(FocusedWindow) -> Fut,
    Fut: Future<Output = FocusTrackerResult<()>>,
{
    let mut prev_state = FocusState::default();
    let mut icon_cache = IconCache::new(config.icon_cache_capacity);

    loop {
        if should_stop(stop_signal) {
            tracing::debug!("Stop signal received, exiting focus tracking loop");
            break;
        }

        match utils::get_frontmost_window_basic_info() {
            Ok(mut window) => {
                if prev_state.has_changed(&window) {
                    if let Some(cached) = icon_cache.get(&window.process_name) {
                        window.icon = Some(Arc::clone(cached));
                    } else {
                        match utils::fetch_icon_for_pid(
                            window.process_id.cast_signed(),
                            &config.icon,
                        ) {
                            Ok(Some(icon)) => {
                                let icon = Arc::new(icon);
                                icon_cache.insert(window.process_name.clone(), Arc::clone(&icon));
                                window.icon = Some(icon);
                            }
                            Ok(None) => {}
                            Err(e) => tracing::debug!("Error fetching icon: {e}"),
                        }
                    }
                    prev_state.update_from(&window);
                    on_focus(window).await?;
                }
            }
            Err(e) => {
                tracing::debug!("Error getting window info: {e}");
            }
        }

        tokio::time::sleep(config.poll_interval).await;
    }

    Ok(())
}
