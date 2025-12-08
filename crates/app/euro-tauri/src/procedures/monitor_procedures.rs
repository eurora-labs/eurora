use euro_vision::{capture_monitor_by_id, rgb_to_base64};

#[taurpc::procedures(
    path = "monitor",
    export_to = "../../../apps/desktop/src/lib/bindings/bindings.ts"
)]
pub trait MonitorApi {
    async fn capture_monitor(monitor_id: String) -> Result<String, String>;
}

#[derive(Clone)]
pub struct MonitorApiImpl;

#[taurpc::resolvers]
impl MonitorApi for MonitorApiImpl {
    async fn capture_monitor(self, monitor_id: String) -> Result<String, String> {
        let image = capture_monitor_by_id(&monitor_id)
            .map_err(|e| format!("Failed to capture monitor: {}", e))?;
        let image = image::DynamicImage::ImageRgba8(image).to_rgb8();

        rgb_to_base64(image).map_err(|e| format!("Failed to encode image: {}", e))
    }
}
