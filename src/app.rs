#![allow(non_snake_case)]

use dioxus::prelude::*;

static CSS: Asset = asset!("/assets/styles.css");

#[derive(Clone, Copy, PartialEq)]
enum DeviceState {
    Ready,
    Running,
    Stopped,
}

struct Phone {
    name: &'static str,
    model: &'static str,
    serial: &'static str,
    state: DeviceState,
}

pub fn App() -> Element {
    let mut devices = use_signal(|| {
        vec![
            Phone {
                name: "Pixel 7 Pro",
                model: "Google · Android 14",
                serial: "2A161FDH3007",
                state: DeviceState::Ready,
            },
            Phone {
                name: "Galaxy S23",
                model: "Samsung · Android 14",
                serial: "R5CT31A8K0X",
                state: DeviceState::Ready,
            },
            Phone {
                name: "Xiaomi 13",
                model: "Xiaomi · Android 13",
                serial: "d5c8e2a1",
                state: DeviceState::Ready,
            },
        ]
    });
    let mut settings_open = use_signal(|| false);
    let mut adb_path = use_signal(|| String::from("adb"));
    let mut refresh_interval = use_signal(|| String::from("5"));
    let mut app_icon = use_signal(|| String::from("phone"));
    let mut workflow_open = use_signal(|| false);
    let mut workflow_name = use_signal(|| String::from("Đăng nhập tài khoản"));
    let mut node_class = use_signal(String::new);
    let mut node_text = use_signal(String::new);
    let mut node_path = use_signal(String::new);
    let mut workflow_steps = use_signal(Vec::<String>::new);

    let ready_count = devices
        .read()
        .iter()
        .filter(|device| device.state == DeviceState::Ready)
        .count();
    let running_count = devices
        .read()
        .iter()
        .filter(|device| device.state == DeviceState::Running)
        .count();
    let any_running = running_count > 0;

    rsx! {
        link { rel: "stylesheet", href: CSS }
        main { class: "app-shell",
            header { class: "topbar",
                a { class: "brand", href: "#",
                    span { class: "brand-mark", "C" }
                    span { class: "brand-name", "Control", span { "Phone" } }
                }
                div { class: "topbar-right",
                    span { class: "connection-indicator",
                        span { class: "connection-dot" }
                        "Giao diện điều khiển"
                    }
                    button {
                        class: "avatar-button",
                        title: "Tài khoản",
                        "CP"
                    }
                }
            }

            section { class: "page-heading",
                div {
                    p { class: "eyebrow", "TRUNG TÂM ĐIỀU KHIỂN" }
                    h1 { "Quản lý thiết bị" }
                    p { class: "page-description", "Theo dõi và điều khiển các điện thoại trong phiên làm việc của bạn." }
                }
                div { class: "session-chip",
                    span { class: if any_running { "session-dot session-running" } else { "session-dot session-stopped" } }
                    if any_running { "Phiên đang chạy" } else { "Phiên đã dừng" }
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
                            devices.with_mut(|items| {
                                for device in items {
                                    device.state = DeviceState::Running;
                                }
                            });
                        },
                        span { class: "action-icon", "▶" }
                        span { class: "action-copy", strong { "Bắt đầu tất cả" } small { "Chạy trên mọi thiết bị" } }
                    }
                    button {
                        class: "action-button action-stop",
                        disabled: !any_running,
                        onclick: move |_| {
                            devices.with_mut(|items| {
                                for device in items {
                                    device.state = DeviceState::Stopped;
                                }
                            });
                        },
                        span { class: "action-icon", "■" }
                        span { class: "action-copy", strong { "Dừng tất cả" } small { "Dừng phiên hiện tại" } }
                    }
                    button {
                        class: "action-button action-reset",
                        onclick: move |_| {
                            devices.with_mut(|items| {
                                for device in items {
                                    device.state = DeviceState::Ready;
                                }
                            });
                        },
                        span { class: "action-icon", "↻" }
                        span { class: "action-copy", strong { "Đặt lại tất cả" } small { "Đưa thiết bị về sẵn sàng" } }
                    }
                    button {
                        class: "action-button action-settings",
                        disabled: any_running,
                        onclick: move |_| settings_open.set(true),
                        span { class: "action-icon", "⚙" }
                        span { class: "action-copy", strong { "Cài đặt" } small { "Cấu hình hệ thống" } }
                    }
                    button {
                        class: if workflow_open() { "action-button action-workflow selected" } else { "action-button action-workflow" },
                        onclick: move |_| workflow_open.toggle(),
                        span { class: "action-icon", "⌘" }
                        span { class: "action-copy", strong { if workflow_open() { "Đóng workflow" } else { "Workflow mới" } } small { "Khai báo quy trình tự động" } }
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
                            p { class: "section-description", "Khai báo quy trình và tiêu chí tìm node trên giao diện điện thoại." }
                        }
                        span { class: "workflow-badge", "WORKFLOW MỚI" }
                    }
                    div { class: "workflow-form",
                        label { class: "field field-wide",
                            span { "Tên workflow" }
                            input {
                                value: "{workflow_name}",
                                placeholder: "Ví dụ: Đăng nhập tài khoản",
                                oninput: move |event| workflow_name.set(event.value())
                            }
                        }
                        div { class: "selector-heading",
                            div {
                                h3 { "Bước và bộ chọn node" }
                                p { "Dùng một hoặc nhiều thuộc tính để xác định phần tử cần thao tác." }
                            }
                            span { class: "step-counter", "{workflow_steps.read().len()} BƯỚC" }
                        }
                        div { class: "selector-fields",
                            label { class: "field",
                                span { "Class / resource-id" }
                                input {
                                    value: "{node_class}",
                                    placeholder: "com.app:id/login_button",
                                    oninput: move |event| node_class.set(event.value())
                                }
                            }
                            label { class: "field",
                                span { "Nội dung text" }
                                input {
                                    value: "{node_text}",
                                    placeholder: "Đăng nhập",
                                    oninput: move |event| node_text.set(event.value())
                                }
                            }
                            label { class: "field field-wide",
                                span { "Đường dẫn node (path)" }
                                input {
                                    value: "{node_path}",
                                    placeholder: "Ví dụ: hierarchy/window[0]/button[1]",
                                    oninput: move |event| node_path.set(event.value())
                                }
                            }
                        }
                        div { class: "workflow-footer",
                            p { class: "form-hint", "Nhập ít nhất một thuộc tính để thêm bước." }
                            button {
                                class: "primary-button",
                                onclick: move |_| {
                                    let class_value = node_class.read().trim().to_string();
                                    let text_value = node_text.read().trim().to_string();
                                    let path_value = node_path.read().trim().to_string();
                                    let parts = [
                                        if class_value.is_empty() { None } else { Some(format!("class: {class_value}")) },
                                        if text_value.is_empty() { None } else { Some(format!("text: {text_value}")) },
                                        if path_value.is_empty() { None } else { Some(format!("path: {path_value}")) },
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
                                for (index, step) in workflow_steps.read().iter().enumerate() {
                                    div { class: "workflow-step", key: "{index}",
                                        span { class: "step-number", "{index + 1}" }
                                        span { class: "step-description", "{step}" }
                                        button {
                                            class: "remove-step",
                                            title: "Xóa bước",
                                            onclick: move |_| workflow_steps.with_mut(|steps| { steps.remove(index); }),
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
                        p { class: "section-description", "Quản lý trạng thái và tác vụ cho từng thiết bị riêng lẻ." }
                    }
                    div { class: "device-summary",
                        span { i { class: "summary-dot summary-online" } "{devices.read().len()} thiết bị" }
                        span { i { class: "summary-dot summary-ready" } "{ready_count} sẵn sàng" }
                        span { i { class: "summary-dot summary-running" } "{running_count} đang chạy" }
                    }
                }

                div { class: "device-table",
                    div { class: "table-header",
                        span { "THIẾT BỊ" }
                        span { "SERIAL" }
                        span { "TRẠNG THÁI" }
                        span { class: "table-actions-label", "ĐIỀU KHIỂN" }
                    }
                    for (index, device) in devices.read().iter().enumerate() {
                        article { class: "device-row", key: "{index}",
                            div { class: "device-identity",
                                span { class: "phone-icon",
                                    svg { view_box: "0 0 24 24", "aria-hidden": "true",
                                        rect { x: "6", y: "2.5", width: "12", height: "19", rx: "2.5" }
                                        path { d: "M10 5h4M11 18.5h2" }
                                    }
                                }
                                span { class: "device-copy",
                                    strong { "{device.name}" }
                                    small { "{device.model}" }
                                }
                            }
                            code { class: "device-serial", "{device.serial}" }
                            div { class: "device-status",
                                {match device.state {
                                    DeviceState::Ready => rsx! { span { class: "status-pill status-ready", span { class: "status-dot" } "Sẵn sàng" } },
                                    DeviceState::Running => rsx! { span { class: "status-pill status-running", span { class: "status-dot" } "Đang chạy" } },
                                    DeviceState::Stopped => rsx! { span { class: "status-pill status-stopped", span { class: "status-dot" } "Đã dừng" } },
                                }}
                            }
                            div { class: "device-actions",
                                button {
                                    class: "device-button device-stop",
                                    title: "Dừng thiết bị",
                                    disabled: device.state != DeviceState::Running,
                                    onclick: move |_| {
                                        devices.with_mut(|items| items[index].state = DeviceState::Stopped);
                                    },
                                    "■" span { "Dừng" }
                                }
                                button {
                                    class: "device-button device-start",
                                    title: "Bắt đầu thiết bị",
                                    disabled: device.state == DeviceState::Running,
                                    onclick: move |_| {
                                        devices.with_mut(|items| items[index].state = DeviceState::Running);
                                    },
                                    "▶" span { "Chạy" }
                                }
                                button {
                                    class: "device-button device-reset",
                                    title: "Đặt lại thiết bị",
                                    onclick: move |_| {
                                        devices.with_mut(|items| items[index].state = DeviceState::Ready);
                                    },
                                    "↻" span { "Đặt lại" }
                                }
                            }
                        }
                    }
                }
                p { class: "demo-note", span { "i" } "Danh sách hiện là dữ liệu giao diện mẫu; kết nối ADB sẽ được tích hợp riêng." }
            }

            footer { class: "app-footer",
                span { "CONTROL PHONE" }
                span { "Quản lý thiết bị Android · Phiên làm việc cục bộ" }
            }
        }

        if settings_open() {
            div { class: "modal-backdrop", onclick: move |_| settings_open.set(false),
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
                            p { "Cấu hình tùy chọn cho phiên điều khiển." }
                        }
                        button { class: "modal-close", title: "Đóng", onclick: move |_| settings_open.set(false), "×" }
                    }
                    div { class: "settings-fields",
                        label { class: "field",
                            span { "Đường dẫn ADB" }
                            input {
                                value: "{adb_path}",
                                placeholder: "Đường dẫn đến adb",
                                oninput: move |event| adb_path.set(event.value())
                            }
                            small { "Để trống nếu adb đã có trong PATH." }
                        }
                        label { class: "field",
                            span { "Thời gian làm mới (giây)" }
                            input {
                                r#type: "number",
                                value: "{refresh_interval}",
                                min: "1",
                                oninput: move |event| refresh_interval.set(event.value())
                            }
                        }
                        label { class: "field",
                            span { "Biểu tượng ứng dụng" }
                            select {
                                value: "{app_icon}",
                                onchange: move |event| app_icon.set(event.value()),
                                option { value: "phone", "Điện thoại" }
                                option { value: "robot", "Android" }
                                option { value: "devices", "Thiết bị" }
                            }
                            small { "Chọn biểu tượng hiển thị cho ứng dụng điều khiển." }
                        }
                    }
                    div { class: "modal-footer",
                        button { class: "secondary-button", onclick: move |_| settings_open.set(false), "Hủy" }
                        button { class: "primary-button", onclick: move |_| settings_open.set(false), "Lưu cài đặt" }
                    }
                }
            }
        }
    }
}
