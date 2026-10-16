
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use adb_core::{TaskStatusEvent, TaskEventPublisher};

struct AppState {
    publisher: TaskEventPublisher
}


#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (tx, mut rx) = mpsc::channel::<TaskStatusEvent>(500);

    let publisher = TaskEventPublisher::new(tx);

    let engine_publisher = publisher.clone();

    tauri::Builder::default()
        .manage(AppState { publisher })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .setup(|app| {
            let app_handle = app.handle().clone();

            tokio::spawn(async move {
                while let Some(event) = rx.recv().await  {
                    let _ = app_handle.emit("task-status-event", event);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
