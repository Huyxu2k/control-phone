use adb_core::TaskStatusEvent;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

pub fn forward_task_status_events(
    app_handle: AppHandle,
    mut receiver: mpsc::Receiver<TaskStatusEvent>,
) {
    tauri::async_runtime::spawn(async move {
        while let Some(event) = receiver.recv().await {
            let serial = event.serial.clone();
            if let Err(error) = app_handle.emit("task-status-event", event) {
                tracing::error!(
                    serial,
                    %error,
                    "Failed to forward task status event to the UI"
                );
            }
        }

        tracing::warn!("Task status event channel closed");
    });
}
