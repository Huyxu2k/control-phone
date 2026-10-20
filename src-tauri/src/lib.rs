mod event_bridge_app;

use adb_core::{TaskEventPublisher, TaskStatus, TaskStatusEvent};
use rand::Rng;
use serde::Serialize;
use std::time::Duration;
use tauri::Manager;
use tokio::sync::mpsc;

#[derive(Clone, Serialize)]
struct MockDevice {
    name: String,
    model: String,
    serial: String,
}

fn create_mock_devices() -> Vec<MockDevice> {
    (1..=10)
        .map(|index| MockDevice {
            name: format!("Điện thoại mẫu {index}"),
            model: format!("Android · Thiết bị mẫu {index}"),
            serial: format!("MOCK-DEVICE-{index:02}"),
        })
        .collect()
}

#[tauri::command]
fn get_mock_devices() -> Vec<MockDevice> {
    create_mock_devices()
}

async fn run_demo_task_status_events(publisher: TaskEventPublisher) {
    let devices = create_mock_devices();
    let mut event_index = 0usize;

    loop {
        let device_index = rand::rng().random_range(0..devices.len());
        let status_index = rand::rng().random_range(0..8);
        let action_index = rand::rng().random_range(0..3);
        let device = &devices[device_index];
        let status = match status_index {
            0 | 1 | 2 | 3 => TaskStatus::Running,
            4 => TaskStatus::Success,
            5 => TaskStatus::Failed,
            6 => TaskStatus::Skipped,
            _ => TaskStatus::Cancelled,
        };
        let action_names = ["Mở ứng dụng", "Kiểm tra kết nối", "Hoàn tất thao tác"];
        let task_index = event_index / devices.len() + 1;
        let mut event = TaskStatusEvent::new(
            "demo-workflow".to_string(),
            device.serial.clone(),
            format!("demo-task-{task_index}"),
            format!("Tác vụ demo {task_index}"),
            task_index,
            status.clone(),
        );
        event.action_index = Some(action_index + 1);
        event.action_name = Some(action_names[action_index].to_string());
        event.message = Some(format!(
            "{} · {}",
            device.name,
            match &status {
                TaskStatus::Running => "Đang thực thi",
                TaskStatus::Success => "Hoàn tất",
                TaskStatus::Failed => "Thất bại",
                TaskStatus::Skipped => "Đã bỏ qua",
                TaskStatus::Cancelled => "Đã hủy",
            }
        ));
        if matches!(status, TaskStatus::Failed) {
            event.error = Some("Lỗi demo giả lập".to_string());
        }
        publisher.publish(event);

        event_index += 1;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (tx, rx) = mpsc::channel::<TaskStatusEvent>(500);
    let publisher = TaskEventPublisher::new(tx);

    tauri::Builder::default()
        .manage(publisher)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_mock_devices])
        .setup(|app| {
            event_bridge_app::forward_task_status_events(app.handle().clone(), rx);

            let publisher = app.state::<TaskEventPublisher>().inner().clone();
            tauri::async_runtime::spawn(async move {
                run_demo_task_status_events(publisher).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
