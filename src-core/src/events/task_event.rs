use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Running,
    Success,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskStatusEvent {
    pub workflow_id: String,
    pub serial: String,
    pub task_id: String,
    pub task_name: String,
    pub task_index: usize,
    pub status: TaskStatus,
    pub action_index: Option<usize>,
    pub action_name: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub timestamp_ms: u64,
}

impl TaskStatusEvent {
    pub fn new(
        workflow_id: String,
        serial: String,
        task_id: String,
        task_name: String,
        task_index: usize,
        status: TaskStatus,
    ) -> Self {
        TaskStatusEvent {
            workflow_id,
            serial,
            task_id,
            task_name,
            task_index,
            status,
            action_index: None,
            action_name: None,
            message: None,
            error: None,
            timestamp_ms: now_ms(),
        }
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_millis() as u64
}
