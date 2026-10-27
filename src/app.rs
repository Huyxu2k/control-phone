#![allow(non_snake_case)]

use dioxus::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use chrono::Local;

use crate::util::{invoke, listen, EventListener};

static CSS: Asset = asset!("/assets/styles.css");

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum TaskStatus {
    Running,
    Success,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Clone, Deserialize)]
struct TaskStatusEvent {
    workflow_id: String,
    serial: String,
    task_id: String,
    task_name: String,
    status: TaskStatus,
    action_index: Option<usize>,
    action_name: Option<String>,
    message: Option<String>,
    timestamp_ms: u64,
}

fn task_status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Running => "Đang chạy",
        TaskStatus::Success => "Hoàn tất",
        TaskStatus::Failed => "Thất bại",
        TaskStatus::Skipped => "Đã bỏ qua",
        TaskStatus::Cancelled => "Đã hủy",
    }
}

fn task_status_class(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Running => "status-running",
        TaskStatus::Success => "status-success",
        TaskStatus::Failed => "status-failed",
        TaskStatus::Skipped | TaskStatus::Cancelled => "status-inactive",
    }
}

#[derive(Deserialize)]
struct MockDevice {
    serial: String,
    name: String,
    model: String,
    task_name: Option<String>,
    action_name: Option<String>,
    message: Option<String>,
    timestamp_ms: Option<String>,
    task_status: Option<TaskStatus>,
    #[serde(skip)]
    state: DeviceState,
}

#[derive(Clone, Copy, Default, PartialEq, Deserialize)]
enum DeviceState {
    #[default]
    Ready,
    Running,
    Stopped,
}

async fn listen_task_status(handler: impl FnMut(TaskStatusEvent) + 'static) -> Result<EventListener, String> {
    listen("task-status-event", handler).await
}

pub fn App() -> Element {
    let mut devices = use_signal(Vec::<MockDevice>::new);
    let mut task_status_by_serial = use_signal(|| HashMap::<String, TaskStatusEvent>::new());
    let mut task_listener = use_signal(|| None::<EventListener>);
    use_hook(move || {
        spawn(async move {
            let result = listen_task_status(move |mut event: TaskStatusEvent| {
            let serial = event.serial.clone();
            let mut by_serial = task_status_by_serial.write();

            if let Some(current) = by_serial.get(&serial) {
                // event cũ hơn -> bỏ qua
                if event.timestamp_ms < current.timestamp_ms {
                    return;
                }
                let same_task = current.workflow_id == event.workflow_id && current.task_id == event.task_id;
                if same_task {
                    if event.action_index.is_none() {
                        event.action_index = current.action_index;
                    }
                    if event.action_name.is_none() {
                        event.action_name.clone_from(&current.action_name);
                    }
                }
            }
            
            // cập nhật device tương ứng
            devices.with_mut(|items| {
                if let Some(device) = items.iter_mut().find(|d| d.serial == serial) {
                    device.task_name = Some(event.task_name.clone());
                    device.task_status = Some(event.status);
                    device.action_name = event.action_name.clone();
                    device.message = event.message.clone();
                    device.timestamp_ms = Some(Local::now().format("%d-%m-%Y %H:%M:%S").to_string()); //Some(event.timestamp_ms);
                }
            });

            by_serial.insert(serial, event);
        })
        .await;

            match result {
                Ok(listener) => task_listener.set(Some(listener)),
                Err(e) => dioxus_logger::tracing::error!(
                    "Failed to listen for task status events: {e}"
                ),
            }
        });

        spawn(async move {
            match invoke::<_, Vec<MockDevice>>("get_mock_devices", &()).await {
                    Ok(mock_devices) => {
                        let latest_task_status = task_status_by_serial.peek();
                        devices.set(
                            mock_devices
                                .into_iter()
                                .map(|device| {
                                    let task = latest_task_status.get(&device.serial);
                                    MockDevice {
                                        task_name: task
                                            .map(|t| t.task_name.clone())
                                            .or(device.task_name),
                                        action_name: match task {
                                            Some(t) => t.action_name.clone(),
                                            None => device.action_name,
                                        },
                                        message: match task {
                                            Some(t) => t.message.clone(),
                                            None => device.message,
                                        },
                                        timestamp_ms: None,//task.map(|t| t.timestamp_ms).or(device.timestamp_ms),
                                        task_status: task.map(|t| t.status),
                                        state: DeviceState::Ready,
                                        ..device // name, model, serial giữ nguyên
                                    }
                                })
                                .collect(),
                        );
                    }
                    Err(error) => {
                        dioxus_logger::tracing::error!("Failed to load mock devices: {error}");
                    }
                }
            });
    });
    
    let mut settings_open = use_signal(|| false);
    let mut adb_path = use_signal(|| String::from("adb"));
    let mut auto_discovery = use_signal(|| true);
    let mut device_poll_interval_ms = use_signal(|| String::from("1000"));
    let mut max_concurrent_devices = use_signal(|| String::from("5"));
    let mut action_timeout_seconds = use_signal(|| String::from("30"));
    let mut retry_count = use_signal(|| String::from("3"));
    let mut worker_queue_size = use_signal(|| String::from("100"));
    let mut settings_error = use_signal(String::new);
    let mut workflow_open = use_signal(|| false);
    let mut workflow_name = use_signal(|| String::from("Đăng nhập tài khoản"));
    let mut node_class = use_signal(String::new);
    let mut node_text = use_signal(String::new);
    let mut node_path = use_signal(String::new);
    let mut workflow_steps = use_signal(Vec::<String>::new);

    let ready_count = devices
        .read()
        .iter()
        .filter(|device| {
            device.state == DeviceState::Ready && device.task_status != Some(TaskStatus::Running)
        })
        .count();
    let running_count = devices
        .read()
        .iter()
        .filter(|device| {
            device.state == DeviceState::Running || device.task_status == Some(TaskStatus::Running)
        })
        .count();
    let any_running = running_count > 0;

    rsx! {
        link { rel: "stylesheet", href: CSS }
        main { class: "app-shell",
            header { class: "topbar",
                a { class: "brand", href: "#",
                    span { class: "brand-mark", "C" }
                    span { class: "brand-name",
                        "Control"
                        span { "MockDevice" }
                    }
                }
                div { class: "topbar-right",
                    span { class: "connection-indicator",
                        span { class: "connection-dot" }
                        "Giao diện điều khiển"
                    }
                    button { class: "avatar-button", title: "Tài khoản", "CP" }
                }
            }

            section { class: "page-heading",
                div {
                    p { class: "eyebrow", "TRUNG TÂM ĐIỀU KHIỂN" }
                    h1 { "Quản lý thiết bị" }
                    p { class: "page-description",
                        "Theo dõi và điều khiển các điện thoại trong phiên làm việc của bạn."
                    }
                }
                div { class: "session-chip",
                    span { class: if any_running { "session-dot session-running" } else { "session-dot session-stopped" } }
                    if any_running {
                        "Phiên đang chạy"
                    } else {
                        "Phiên đã dừng"
                    }
                }
            }

            section { class: "control-panel",
                div { class: "panel-heading",
                    div {
                        h2 { "Điều khiển hệ thống" }
                        p { "Thao tác nhanh trên toàn bộ thiết bị" }
                    }
                    span { class: "panel-caption", "TÁC VỤ TOÀN CỤC" }
                }
                div { class: "global-actions",
                    button {
                        class: "action-button action-start",
                        disabled: any_running,
                        onclick: move |_| {
                            devices
                                .with_mut(|items| {
                                    for device in items {
                                        device.state = DeviceState::Running;
                                        device.task_status = None;
                                    }
                                });
                        },
                        span { class: "action-icon", "▶" }
                        span { class: "action-copy",
                            strong { "Bắt đầu tất cả" }
                            small { "Chạy trên mọi thiết bị" }
                        }
                    }
                    button {
                        class: "action-button action-stop",
                        disabled: !any_running,
                        onclick: move |_| {
                            devices
                                .with_mut(|items| {
                                    for device in items {
                                        device.state = DeviceState::Stopped;
                                        device.task_status = None;
                                    }
                                });
                        },
                        span { class: "action-icon", "■" }
                        span { class: "action-copy",
                            strong { "Dừng tất cả" }
                            small { "Dừng phiên hiện tại" }
                        }
                    }
                    button {
                        class: "action-button action-reset",
                        onclick: move |_| {
                            devices
                                .with_mut(|items| {
                                    for device in items {
                                        device.state = DeviceState::Ready;
                                        device.task_status = None;
                                    }
                                });
                        },
                        span { class: "action-icon", "↻" }
                        span { class: "action-copy",
                            strong { "Đặt lại tất cả" }
                            small { "Đưa thiết bị về sẵn sàng" }
                        }
                    }
                    button {
                        class: "action-button action-settings",
                        disabled: any_running,
                        onclick: move |_| settings_open.set(true),
                        span { class: "action-icon", "⚙" }
                        span { class: "action-copy",
                            strong { "Cài đặt" }
                            small { "Cấu hình hệ thống" }
                        }
                    }
                    button {
                        class: if workflow_open() { "action-button action-workflow selected" } else { "action-button action-workflow" },
                        onclick: move |_| workflow_open.toggle(),
                        span { class: "action-icon", "⌘" }
                        span { class: "action-copy",
                            strong {
                                if workflow_open() {
                                    "Đóng workflow"
                                } else {
                                    "Workflow mới"
                                }
                            }
                            small { "Khai báo quy trình tự động" }
                        }
                    }
                }
                if any_running {
                    div { class: "running-note",
                        span { class: "running-pulse" }
                        "Hệ thống đang chạy. Cài đặt chỉ khả dụng khi phiên đã dừng."
                    }
                }
            }

            if workflow_open() {
                section { class: "workflow-panel",
                    div { class: "section-heading",
                        div {
                            p { class: "eyebrow", "TỰ ĐỘNG HÓA" }
                            h2 { "Tạo workflow" }
                            p { class: "section-description",
                                "Khai báo quy trình và tiêu chí tìm node trên giao diện điện thoại."
                            }
                        }
                        span { class: "workflow-badge", "WORKFLOW MỚI" }
                    }
                    div { class: "workflow-form",
                        label { class: "field field-wide",
                            span { "Tên workflow" }
                            input {
                                value: "{workflow_name}",
                                placeholder: "Ví dụ: Đăng nhập tài khoản",
                                oninput: move |event| workflow_name.set(event.value()),
                            }
                        }
                        div { class: "selector-heading",
                            div {
                                h3 { "Bước và bộ chọn node" }
                                p {
                                    "Dùng một hoặc nhiều thuộc tính để xác định phần tử cần thao tác."
                                }
                            }
                            span { class: "step-counter", "{workflow_steps.read().len()} BƯỚC" }
                        }
                        div { class: "selector-fields",
                            label { class: "field",
                                span { "Class / resource-id" }
                                input {
                                    value: "{node_class}",
                                    placeholder: "com.app:id/login_button",
                                    oninput: move |event| node_class.set(event.value()),
                                }
                            }
                            label { class: "field",
                                span { "Nội dung text" }
                                input {
                                    value: "{node_text}",
                                    placeholder: "Đăng nhập",
                                    oninput: move |event| node_text.set(event.value()),
                                }
                            }
                            label { class: "field field-wide",
                                span { "Đường dẫn node (path)" }
                                input {
                                    value: "{node_path}",
                                    placeholder: "Ví dụ: hierarchy/window[0]/button[1]",
                                    oninput: move |event| node_path.set(event.value()),
                                }
                            }
                        }
                        div { class: "workflow-footer",
                            p { class: "form-hint",
                                "Nhập ít nhất một thuộc tính để thêm bước."
                            }
                            button {
                                class: "primary-button",
                                onclick: move |_| {
                                    let class_value = node_class.read().trim().to_string();
                                    let text_value = node_text.read().trim().to_string();
                                    let path_value = node_path.read().trim().to_string();
                                    let parts = [
                                        if class_value.is_empty() {
                                            None
                                        } else {
                                            Some(format!("class: {class_value}"))
                                        },
                                        if text_value.is_empty() {
                                            None
                                        } else {
                                            Some(format!("text: {text_value}"))
                                        },
                                        if path_value.is_empty() {
                                            None
                                        } else {
                                            Some(format!("path: {path_value}"))
                                        },
                                    ];
                                    let description = parts.into_iter().flatten().collect::<Vec<_>>().join(" · ");
                                    if !description.is_empty() {
                                        workflow_steps.with_mut(|steps| steps.push(description));
                                        node_class.set(String::new());
                                        node_text.set(String::new());
                                        node_path.set(String::new());
                                    }
                                },
                                span { "＋" }
                                "Thêm bước"
                            }
                        }
                        if !workflow_steps.read().is_empty() {
                            div { class: "workflow-steps",
                                for (index , step) in workflow_steps.read().iter().enumerate() {
                                    div {
                                        class: "workflow-step",
                                        key: "{index}",
                                        span { class: "step-number", "{index + 1}" }
                                        span { class: "step-description", "{step}" }
                                        button {
                                            class: "remove-step",
                                            title: "Xóa bước",
                                            onclick: move |_| {
                                                workflow_steps
                                                    .with_mut(|steps| {
                                                        steps.remove(index);
                                                    })
                                            },
                                            "×"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            section { class: "devices-section",
                div { class: "section-heading devices-heading",
                    div {
                        p { class: "eyebrow", "THIẾT BỊ" }
                        h2 { "Điện thoại của bạn" }
                        p { class: "section-description",
                            "Quản lý trạng thái và tác vụ cho từng thiết bị riêng lẻ."
                        }
                    }
                    div { class: "device-summary",
                        span {
                            i { class: "summary-dot summary-online" }
                            "{devices.read().len()} thiết bị"
                        }
                        span {
                            i { class: "summary-dot summary-ready" }
                            "{ready_count} sẵn sàng"
                        }
                        span {
                            i { class: "summary-dot summary-running" }
                            "{running_count} đang chạy"
                        }
                    }
                }

                div { class: "device-table",
                    div { class: "table-header",
                        span { "THIẾT BỊ" }
                        span { "SERIAL" }
                        span { "TÁC VỤ GẦN NHẤT" }
                        span { "TRẠNG THÁI" }
                        span { class: "table-actions-label", "ĐIỀU KHIỂN" }
                    }
                    for (index , device) in devices.read().iter().enumerate() {
                        article { class: "device-row", key: "{index}",
                            div { class: "device-identity",
                                span { class: "MockDevice-icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        "aria-hidden": "true",
                                        rect {
                                            x: "6",
                                            y: "2.5",
                                            width: "12",
                                            height: "19",
                                            rx: "2.5",
                                        }
                                        path { d: "M10 5h4M11 18.5h2" }
                                    }
                                }
                                span { class: "device-copy",
                                    strong { "{device.name}" }
                                    small { "{device.model}" }
                                }
                            }
                            code { class: "device-serial", "{device.serial}" }
                            div { class: "device-task",
                                if let Some(task_name) = &device.task_name {
                                    strong { "{task_name}" }
                                    if let Some(action_name) = &device.action_name {
                                        small { "Thao tác: {action_name}" }
                                    }
                                    // if let Some(message) = &device.message {
                                    //     small { class: "device-task-message", "{message}" }
                                    // }
                                    if let Some(timestamp_ms) = &device.timestamp_ms {
                                        small { class: "device-task-timestamp",
                                            "Cập nhật: {timestamp_ms} ms"
                                        }
                                    }
                                } else {
                                    small { class: "device-task-empty", "Chưa có tác vụ" }
                                }
                            }
                            div { class: "device-status",
                                {
                                    if let Some(task_status) = device.task_status {
                                        rsx! {
                                            span { class: "status-pill {task_status_class(task_status)}",
                                                span { class: "status-dot" }
                                                "{task_status_label(task_status)}"
                                            }
                                        }
                                    } else {
                                        match device.state {
                                            DeviceState::Ready => rsx! {
                                                span { class: "status-pill status-ready",
                                                    span { class: "status-dot" }
                                                    "Sẵn sàng"
                                                }
                                            },
                                            DeviceState::Running => rsx! {
                                                span { class: "status-pill status-running",
                                                    span { class: "status-dot" }
                                                    "Đang chạy"
                                                }
                                            },
                                            DeviceState::Stopped => rsx! {
                                                span { class: "status-pill status-stopped",
                                                    span { class: "status-dot" }
                                                    "Đã dừng"
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                            div { class: "device-actions",
                                button {
                                    class: "device-button device-stop",
                                    title: "Dừng thiết bị",
                                    disabled: device.state != DeviceState::Running,
                                    onclick: move |_| {
                                        devices
                                            .with_mut(|items| {
                                                items[index].state = DeviceState::Stopped;
                                                items[index].task_status = None;
                                            });
                                    },
                                    "■"
                                    span { "Dừng" }
                                }
                                button {
                                    class: "device-button device-start",
                                    title: "Bắt đầu thiết bị",
                                    disabled: device.state == DeviceState::Running,
                                    onclick: move |_| {
                                        devices
                                            .with_mut(|items| {
                                                items[index].state = DeviceState::Running;
                                                items[index].task_status = None;
                                            });
                                    },
                                    "▶"
                                    span { "Chạy" }
                                }
                                button {
                                    class: "device-button device-reset",
                                    title: "Đặt lại thiết bị",
                                    onclick: move |_| {
                                        devices
                                            .with_mut(|items| {
                                                items[index].state = DeviceState::Ready;
                                                items[index].task_status = None;
                                            });
                                    },
                                    "↻"
                                    span { "Đặt lại" }
                                }
                            }
                        }
                    }
                }
                p { class: "demo-note",
                    span { "i" }
                    "Danh sách gồm 10 thiết bị mẫu; tiến độ tác vụ được cập nhật theo serial từ sự kiện backend."
                }
            
            }

            footer { class: "app-footer",
                span { "CONTROL MockDevice" }
                span { "Quản lý thiết bị Android · Phiên làm việc cục bộ" }
            }
        }

        if settings_open() {
            div {
                class: "modal-backdrop",
                onclick: move |_| settings_open.set(false),
                section {
                    class: "settings-modal",
                    role: "dialog",
                    "aria-modal": "true",
                    "aria-labelledby": "settings-title",
                    onclick: move |event| event.stop_propagation(),
                    div { class: "modal-heading",
                        span { class: "modal-icon", "⚙" }
                        div {
                            p { class: "eyebrow", "TÙY CHỈNH" }
                            h2 { id: "settings-title", "Cài đặt hệ thống" }
                            p { "Cấu hình kết nối thiết bị và xử lý tác vụ." }
                        }
                        button {
                            class: "modal-close",
                            title: "Đóng",
                            onclick: move |_| settings_open.set(false),
                            "×"
                        }
                    }
                    form {
                        class: "settings-form",
                        onsubmit: move |event| {
                            event.prevent_default();
                            let valid = device_poll_interval_ms.read().parse::<u64>().is_ok()
                                && max_concurrent_devices.read().parse::<usize>().is_ok()
                                && action_timeout_seconds.read().parse::<u64>().is_ok()
                                && retry_count.read().parse::<u32>().is_ok()
                                && worker_queue_size.read().parse::<usize>().is_ok();
                            if valid {
                                settings_error.set(String::new());
                                settings_open.set(false);
                            } else {
                                settings_error
                                    .set(
                                        "Vui lòng nhập giá trị số hợp lệ cho các trường cấu hình."
                                            .to_string(),
                                    );
                            }
                        },
                        div { class: "settings-fields",
                            label { class: "field",
                                span { "Đường dẫn ADB" }
                                input {
                                    value: "{adb_path}",
                                    placeholder: "Đường dẫn đến adb",
                                    oninput: move |event| adb_path.set(event.value()),
                                }
                            }
                            label { class: "field setting-toggle",
                                span { "Tự động tìm thiết bị" }
                                div { class: "setting-toggle-control",
                                    small {
                                        "Tự động phát hiện thiết bị mới được kết nối."
                                    }
                                    input {
                                        r#type: "checkbox",
                                        checked: auto_discovery(),
                                        onchange: move |event| auto_discovery.set(event.checked()),
                                    }
                                }
                            }
                            label { class: "field",
                                span { "Chu kỳ quét thiết bị (ms)" }
                                input {
                                    r#type: "number",
                                    value: "{device_poll_interval_ms}",
                                    min: "1",
                                    required: true,
                                    oninput: move |event| device_poll_interval_ms.set(event.value()),
                                }
                            }
                            label { class: "field",
                                span { "Số thiết bị chạy đồng thời" }
                                input {
                                    r#type: "number",
                                    value: "{max_concurrent_devices}",
                                    min: "1",
                                    required: true,
                                    oninput: move |event| max_concurrent_devices.set(event.value()),
                                }
                            }
                            label { class: "field",
                                span { "Thời gian chờ tác vụ (giây)" }
                                input {
                                    r#type: "number",
                                    value: "{action_timeout_seconds}",
                                    min: "1",
                                    required: true,
                                    oninput: move |event| action_timeout_seconds.set(event.value()),
                                }
                            }
                            label { class: "field",
                                span { "Số lần thử lại" }
                                input {
                                    r#type: "number",
                                    value: "{retry_count}",
                                    min: "0",
                                    required: true,
                                    oninput: move |event| retry_count.set(event.value()),
                                }
                            }
                            label { class: "field",
                                span { "Kích thước hàng đợi worker" }
                                input {
                                    r#type: "number",
                                    value: "{worker_queue_size}",
                                    min: "1",
                                    required: true,
                                    oninput: move |event| worker_queue_size.set(event.value()),
                                }
                            }
                        }
                        if !settings_error.read().is_empty() {
                            p { class: "settings-error", role: "alert", "{settings_error}" }
                        }
                        div { class: "modal-footer",
                            button {
                                class: "secondary-button",
                                r#type: "button",
                                onclick: move |_| {
                                    settings_error.set(String::new());
                                    settings_open.set(false);
                                },
                                "Hủy"
                            }
                            button { class: "primary-button", r#type: "submit", "Lưu cài đặt" }
                        }
                    }
                }
            }
        }
    }
}
