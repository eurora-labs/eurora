use eur_vision::{capture_monitor_by_id, image_to_base64};

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
        let image = capture_monitor_by_id(monitor_id).unwrap();
        let image = match cfg!(target_os = "linux") {
            true => pollster::block_on(eur_renderer::blur_image(&image, 0.1, 36.0)),
            false => image::DynamicImage::ImageRgba8(image).to_rgb8(),
        };

        Ok(image_to_base64(image).unwrap())
    }
}
