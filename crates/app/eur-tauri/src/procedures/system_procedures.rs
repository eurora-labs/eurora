use eur_activity::ContextChip;
use eur_timeline::TimelineManager;
use tauri::{Emitter, Manager, Runtime};
use tracing::info;

#[taurpc::procedures(path = "system")]
pub trait SystemApi {
    async fn check_grpc_server_connection(server_address: Option<String>)
    -> Result<String, String>;

    async fn list_activities<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<Vec<ContextChip>, String>;

    async fn send_key_to_launcher<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        key: String,
    ) -> Result<(), String>;
}

#[derive(Clone)]
pub struct SystemApiImpl;

#[taurpc::resolvers]
impl SystemApi for SystemApiImpl {
    async fn check_grpc_server_connection(
        self,
        server_address: Option<String>,
    ) -> Result<String, String> {
        let address = server_address.unwrap_or_else(|| "0.0.0.0:50051".to_string());

        info!("Checking connection to gRPC server: {}", address);

        // Try to establish a basic TCP connection to check if the server is listening
        match tokio::net::TcpStream::connect(address.replace("http://", "").replace("https://", ""))
            .await
        {
            Ok(_) => {
                info!("TCP connection successful");
                Ok("Server is reachable".to_string())
            }
            Err(e) => {
                let error_msg = format!("Failed to connect to server: {}", e);
                info!("{}", error_msg);
                Err(error_msg)
            }
        }
    }

    async fn list_activities<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<Vec<ContextChip>, String> {
        let timeline_state: tauri::State<async_mutex::Mutex<TimelineManager>> = app_handle.state();
        let timeline = timeline_state.lock().await;

        // Get all activities from the timeline
        let activities = timeline.get_context_chips().await;

        // Limit to the 5 most recent activities to avoid cluttering the UI
        let limited_activities = activities.into_iter().take(5).collect::<Vec<ContextChip>>();

        Ok(limited_activities)
    }

    async fn send_key_to_launcher<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        key: String,
    ) -> Result<(), String> {
        if let Some(launcher) = app_handle.get_window("launcher") {
            launcher
                .emit("key_event", key)
                .map_err(|e| format!("Failed to send key event: {}", e))?;
        }
        Ok(())
    }
}
