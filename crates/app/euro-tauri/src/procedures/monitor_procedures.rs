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
    async fn capture_monitor(self, _monitor_id: String) -> Result<String, String> {
        todo!()
    }
}
