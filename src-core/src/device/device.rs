
#[derive(Debug, Clone)]
pub enum DeviceState {
    Unknown,
    Offline,
    Online,
}

#[derive(Debug, Clone)]
pub struct AndroidDevice {
    pub serial: String,
    pub state: DeviceState,
}