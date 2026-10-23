#![allow(non_snake_case)]

use dioxus::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use wasm_bindgen::{JsCast, JsValue, closure::Closure, prelude::wasm_bindgen};

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
    timestamp_ms: Option<u64>,
}

#[wasm_bindgen(inline_js = "export async function listenTaskStatus(callback) {
        const listen = window.__TAURI__?.event?.listen;
        if (!listen) throw new Error('Tauri event API is unavailable');
        return await listen('task-status-event', event => callback(event.payload));
    }")]
extern "C" {
    #[wasm_bindgen(catch, js_name = listenTaskStatus)]
    async fn listen_task_status(callback: &js_sys::Function) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen(inline_js = "export async function getMockDevices() {
        const invoke = window.__TAURI__?.core?.invoke;
        if (!invoke) throw new Error('Tauri invoke API is unavailable');
        return await invoke('get_mock_devices');
    }")]
extern "C" {
    #[wasm_bindgen(catch, js_name = getMockDevices)]
    async fn get_mock_devices() -> Result<JsValue, JsValue>;
}

#[derive(Clone, Copy, PartialEq)]
enum DeviceState {
    Ready,
    Running,
    Stopped,
}

struct Phone {
    name: String,
    model: String,
    serial: String,
    task_name: Option<String>,
    action_name: Option<String>,
    message: Option<String>,
    timestamp_ms: Option<u64>,
    task_status: Option<TaskStatus>,
    state: DeviceState,
}

pub fn App() -> Element {
    let mut devices = use_signal(Vec::<Phone>::new);
    let mut task_status_by_serial = use_signal(|| HashMap::<String, TaskStatusEvent>::new());
    use_hook(move || {
        let callback =
            Closure::<dyn FnMut(JsValue)>::new(
                move |payload| match serde_wasm_bindgen::from_value::<TaskStatusEvent>(payload) {
                    Ok(mut event) => {
                        let device_update = task_status_by_serial.with_mut(|by_serial| {
                            let serial = event.serial.clone();
                            if let Some(current) = by_serial.get(&serial) {
                                let same_task = current.workflow_id == event.workflow_id
                                    && current.task_id == event.task_id;
                                let is_newer = event.timestamp_ms >= current.timestamp_ms;
                                if same_task && is_newer {
                                    if event.action_index.is_none() {
                                        event.action_index = current.action_index;
                                    }
                                    if event.action_name.is_none() {
                                        event.action_name.clone_from(&current.action_name);
                                    }
                                }
                                if is_newer {
                                    let update = (
                                        event.serial.clone(),
                                        event.task_name.clone(),
                                        event.status,
                                        event.action_name.clone(),
                                        event.message.clone(),
                                        event.timestamp_ms,
                                    );
                                    by_serial.insert(serial, event);
                                    Some(update)
                                } else {
                                    None
                                }
                            } else {
                                let update = (
                                    event.serial.clone(),
                                    event.task_name.clone(),
                                    event.status,
                                    event.action_name.clone(),
                                    event.message.clone(),
                                    event.timestamp_ms,
                                );
                                by_serial.insert(serial, event);
                                Some(update)
                            }
                        });
                        if let Some((
                            serial,
                            task_name,
                            task_status,
                            action_name,
                            message,
                            timestamp_ms,
                        )) = device_update
                        {
                            devices.with_mut(|items| {
                                if let Some(device) =
                                    items.iter_mut().find(|device| device.serial == serial)
                                {
                                    device.task_name = Some(task_name);
                                    device.task_status = Some(task_status);
                                    device.action_name = action_name;
                                    device.message = message;
                                    device.timestamp_ms = Some(timestamp_ms);
                                }
                            });
                        }
                    }
                    Err(error) => {
                        dioxus_logger::tracing::error!("Invalid task status event: {error}")
                    }
                },
            );
        let callback_ref = callback
            .as_ref()
            .unchecked_ref::<js_sys::Function>()
            .clone();
        callback.forget();
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(error) = listen_task_status(&callback_ref).await {
                dioxus_logger::tracing::error!(
                    "Failed to listen for task status events: {error:?}"
                );
            }
        });
        wasm_bindgen_futures::spawn_local(async move {
            match get_mock_devices().await {
                Ok(value) => match serde_wasm_bindgen::from_value::<Vec<MockDevice>>(value) {
                    Ok(mock_devices) => {
                        let latest_task_status = task_status_by_serial.read();
                        devices.set(
                            mock_devices
                                .into_iter()
                                .map(|device| {
                                    let task = latest_task_status.get(&device.serial);
                                    Phone {
                                        name: device.name,
                                        model: device.model,
                                        serial: device.serial,
                                        task_name: task
                                            .map(|task| task.task_name.clone())
                                            .or(device.task_name),
                                        action_name: if let Some(task) = task {
                                            task.action_name.clone()
                                        } else {
                                            device.action_name
                                        },
                                        message: if let Some(task) = task {
                                            task.message.clone()
                                        } else {
                                            device.message
                                        },
                                        timestamp_ms: task
                                            .map(|task| task.timestamp_ms)
                                            .or(device.timestamp_ms),
                                        task_status: task.map(|task| task.status),
                                        state: DeviceState::Ready,
                                    }
                                })
                                .collect(),
                        );
                    }
                    Err(error) => {
                        dioxus_logger::tracing::error!("Invalid mock device response: {error}")
                    }
                },
                Err(error) => {
                    dioxus_logger::tracing::error!("Failed to load mock devices: {error:?}")
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
                        span { "Phone" }
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
                                span { class: "phone-icon",
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
                                    if let Some(timestamp_ms) = device.timestamp_ms {
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
                span { "CONTROL PHONE" }
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
