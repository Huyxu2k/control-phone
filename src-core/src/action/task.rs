use serde::de;

use super::action::Action;

#[derive(Debug, Clone)]
pub enum DeviceSelector {
    Any,
    Device(String),
    Devices(Vec<String>),
}

impl DeviceSelector {
    pub fn matches(&self, serial: &str) -> bool {
        match self {
            DeviceSelector::Any => true,
            DeviceSelector::Device(device) => device == serial,
            DeviceSelector::Devices(devices) => devices.iter().any(|i| i == serial),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub selector: DeviceSelector,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub selector: DeviceSelector,
    pub tasks: Vec<Task>,
}
