use tokio::sync::mpsc;

use crate::{Action, Task, Workflow};

use super::task_event::{TaskStatus, TaskStatusEvent};

#[derive(Clone)]
pub struct TaskEventPublisher {
    sender: mpsc::Sender<TaskStatusEvent>,
}

impl TaskEventPublisher {
    pub fn new(sender: mpsc::Sender<TaskStatusEvent>) -> Self {
        TaskEventPublisher { sender }
    }

    pub fn publish(&self, event: TaskStatusEvent) {
        if let Err(error) = self.sender.try_send(event) {
            tracing::warn!(?error, "Failed to publish task status event");
        }
    }

    pub fn task_running(&self, Workflow: &Workflow, serial: &str, task: &Task, task_index: usize) {
        let mut event = TaskStatusEvent::new(
            Workflow.id.clone(),
            serial.to_string(),
            task.id.clone(),
            task.name.clone(),
            task_index + 1,
            TaskStatus::Running,
        );

        event.message = Some("Task is running".to_string());
        self.publish(event);
    }

    // pub fn action_running(&self, Workflow: &Workflow, serial: &str, task: &Task, task_index: usize, action: &Action, action_index: usize) {
    //     let mut event = TaskStatusEvent::new(
    //         Workflow.id.clone(),
    //         serial.to_string(),
    //         task.id.clone(),
    //         task.name.clone(),
    //         task_index + 1,
    //         TaskStatus::Running,
    //     );

    //     event.action_index = Some(action_index + 1);
    //     event.action_name = Some(action.value.to_string());
    //     event.message = Some("Action is running".to_string());
    //     self.publish(event);
    // }

    // pub fn task_success(&self, Workflow: &Workflow, serial: &str, task: &Task, task_index: usize) {
    //     let mut event = TaskStatusEvent::new(
    //         Workflow.id.clone(),
    //         serial.to_string(),
    //         task.id.clone(),
    //         task.name.clone(),
    //         task_index + 1,
    //         TaskStatus::Success,
    //     );

    //     event.message = Some("Task completed successfully".to_string());
    //     self.publish(event);
    // }

    // pub fn action_failed(&self, Workflow: &Workflow, serial: &str, task: &Task, task_index: usize, action: &Action, action_index: usize, error_message: String) {
    //     let mut event = TaskStatusEvent::new(
    //         Workflow.id.clone(),
    //         serial.to_string(),
    //         task.id.clone(),
    //         task.name.clone(),
    //         task_index + 1,
    //         TaskStatus::Failed,
    //     );

    //     event.action_index = Some(action_index + 1);
    //     event.action_name = Some(action.value.to_string());
    //     event.error = Some(error_message);
    //     self.publish(event);
    // }
}
