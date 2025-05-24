use eur_vision::{capture_monitor_by_name, image_to_base64};
use image::DynamicImage;

#[taurpc::procedures(
    path = "monitor",
    export_to = "../../../packages/tauri-bindings/src/lib/gen/bindings.ts"
)]
pub trait MonitorApi {
    async fn capture_monitor(monitor_name: String) -> Result<String, String>;
}

#[derive(Clone)]
pub struct MonitorApiImpl;

#[taurpc::resolvers]
impl MonitorApi for MonitorApiImpl {
    async fn capture_monitor(self, monitor_name: String) -> Result<String, String> {
        let image = capture_monitor_by_name(monitor_name).unwrap();
        let image = match cfg!(target_os = "linux") {
            true => pollster::block_on(eur_renderer::blur_image(&image, 0.1, 36.0)),
            false => image::DynamicImage::ImageRgba8(image).to_rgb8(),
        };

        Ok(image_to_base64(image).unwrap())
    }
}
