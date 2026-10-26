mod event_bridge_app;

use adb_core::{Action, DeviceSelector, Task, TaskEventPublisher, TaskStatus, TaskStatusEvent};
use rand::Rng;
use serde::Serialize;
use std::time::Duration;
use tauri::Manager;
use tokio::sync::mpsc;

#[derive(Clone, Serialize)]
struct MockDevice {
    serial: String,
    name: String,
    model: String,
    task_name: Option<String>,
    action_name: Option<String>,
    message: Option<String>,
    timestamp_ms: Option<u64>,
}


fn mock_workflow() -> Vec<Task> {
    vec![
        Task {
            id: "task-login".to_string(),
            name: "Đăng nhập".to_string(),
            selector: DeviceSelector::Any,
            actions: vec![
                Action::LaunchApp {
                    package: "com.example.app".to_string(),
                    activity: "com.example.app.MainActivity".to_string(),
                },
                Action::InputText {
                    text: "username".to_string(),
                },
                Action::TapText {
                    text: "Login".to_string(),
                },
            ],
        },
        Task {
            id: "task-claim-coins".to_string(),
            name: "Nhận xu".to_string(),
            selector: DeviceSelector::Any,
            actions: vec![
                Action::TapText {
                    text: "Menu chính".to_string(),
                },
                Action::TapText {
                    text: "Button A".to_string(),
                },
            ],
        },
        Task {
            id: "task-reply-message".to_string(),
            name: "Rep tin".to_string(),
            selector: DeviceSelector::Any,
            actions: vec![
                Action::TapText {
                    text: "Tin nhắn".to_string(),
                },
                Action::InputText {
                    text: "hahaha".to_string(),
                },
                Action::TapText {
                    text: "Gửi".to_string(),
                },
            ],
        },
    ]
}

fn mock_devices() -> Vec<MockDevice> {
    (1..=10)
        .map(|index| MockDevice {
            serial: format!("MOCK-DEVICE-{index:02}"),
            name: format!("Điện thoại mẫu {index}"),
            model: format!("Android · Thiết bị mẫu {index}"),
            task_name: None,
            action_name: None,
            message: None,
            timestamp_ms: None,
        })
        .collect()
}

#[tauri::command(rename_all = "snake_case")]
fn get_mock_devices() -> Vec<MockDevice> {
    mock_devices()
}

async fn run_demo_task_status_events(publisher: TaskEventPublisher) {
    let devices = mock_devices();
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


/// invoke('greet', { name_value: 'Peter' });
#[tauri::command(rename_all = "snake_case")]
fn greet(name_value: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name_value)
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
