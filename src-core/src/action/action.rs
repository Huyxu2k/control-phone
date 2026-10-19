#[derive(Debug, Clone)]
pub enum Action {
    LaunchApp {
        package: String,
        activity: String,
    },

    Tap {
        x: i32,
        y: i32,
    },

    TapText {
        text: String,
    },

    TapId {
        id: String,
    },

    TapClass {
        class_name: String,
    },

    TapContentDescription {
        content_desc: String,
    },

    InputText {
        text: String,
    },

    Swipe {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        duration_ms: u64,
    },

    Back,

    Home,

    Screenshot,

    DumpUi,

    Wait {
        duration_ms: u64,
    },

    WaitFor {
        //selector: Selector,
        timeout_ms: u64,
        interval_ms: u64,
    },

    SetVolume {
        value: u8,
    },

    Mute,

    WifiEnable,

    WifiDisable,

    Shell {
        command: String,
    },
}

impl Action {
    pub fn name(&self) -> String {
        match self {
            Action::LaunchApp { package, activity } =>  "Launch App".to_string(),
            Action::Tap { x, y } => "Tap".to_string(),
            Action::TapText { text } => "Tap Text".to_string(),
            Action::TapId { id } => "Tap Id".to_string(),
            Action::TapClass { class_name } => "Tap Class".to_string(),
            Action::TapContentDescription { content_desc } => "Tap Content Description".to_string(),
            Action::InputText { text } => "InputText".to_string(),
            Action::Swipe { x1, y1, x2, y2, duration_ms } => "Swipe".to_string(),
            Action::Back => "Back".to_string(),
            Action::Home => "Home".to_string(),
            Action::Screenshot => "Screenshot".to_string(),
            Action::DumpUi => "Dump Ui".to_string(),
            Action::Wait { duration_ms } => "Wait".to_string(),
            Action::WaitFor { timeout_ms, interval_ms } => "WaitFor".to_string(),
            Action::SetVolume { value } => "Set Volume".to_string(),
            Action::Mute => "Mute".to_string(),
            Action::WifiEnable => "Wifi Enable".to_string(),
            Action::WifiDisable => "Wifi Disable".to_string(),
            Action::Shell { command } => "Shell".to_string(),
        }
    }
}