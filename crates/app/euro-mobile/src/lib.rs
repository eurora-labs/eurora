#![allow(
    clippy::used_underscore_binding,
    clippy::module_name_repetitions,
    clippy::too_many_lines
)]

use taurpc::Router;

mod setup;

pub fn build_router() -> Router<tauri::Wry> {
    Router::new().export_config(
        specta_typescript::Typescript::default()
            .bigint(specta_typescript::BigIntExportBehavior::BigInt),
    )
}

pub fn run() {
    dotenv::dotenv().ok();

    let tauri_context = tauri::generate_context!();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime")
        .block_on(async {
            tauri::async_runtime::set(tokio::runtime::Handle::current());

            let router = build_router();

            tauri::Builder::default()
                .plugin(tauri_plugin_os::init())
                .plugin(tauri_plugin_http::init())
                .plugin(tauri_plugin_opener::init())
                .setup(move |app| {
                    setup::init(app)?;
                    Ok(())
                })
                .invoke_handler(router.into_handler())
                .build(tauri_context)
                .expect("Failed to build tauri app")
                .run(|_app_handle, _event| {});
        });
}
