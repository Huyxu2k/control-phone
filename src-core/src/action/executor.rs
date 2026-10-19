use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::{AppError, DeviceController, Result, TaskEventPublisher, TaskStatus, TaskStatusEvent};

use super::{
    action::Action,
    task::{Task, Workflow},
};


pub struct ActionExecutor {
    controller: DeviceController
}

impl ActionExecutor {
    pub fn new(controller: DeviceController) -> Self{
        Self { controller }
    }

    pub async fn execute_task(
         &self,
        workflow: &Workflow,
        task: &Task,
        task_index: usize,
        publisher: &TaskEventPublisher,
        cancel: CancellationToken,
    ) -> Result<()> {
        let serial = self.controller.serial();

        // 1. Task bắt đầu.
        if cancel.is_cancelled() {
            publisher.task_cancelled(
                workflow,
                serial,
                task,
                task_index,
            );

            return Err(AppError::Cancelled);
        }

        publisher.task_running(
            workflow,
            serial,
            task,
            task_index,
        );

        // 2. Thực thi Action theo thứ tự.
        for (action_index, action) in task.actions.iter().enumerate() {
            if cancel.is_cancelled() {
                self.publish_cancelled(
                    workflow,
                    task,
                    task_index,
                    action_index,
                    action,
                    publisher,
                );

                return Err(AppError::Cancelled);
            }

            publisher.action_running(
                workflow,
                serial,
                task,
                task_index,
                action_index,
                action,
            );

            // Action nhận token để những lệnh ADB đang chạy
            // có thể bị hủy khi Workflow bị cancel.
            let action_cancel = cancel.child_token();

            let result = self
                .execute_action(action, action_cancel)
                .await;

            match result {
                Ok(()) => {}

                Err(AppError::Cancelled) => {
                    self.publish_cancelled(
                        workflow,
                        task,
                        task_index,
                        action_index,
                        action,
                        publisher,
                    );

                    return Err(AppError::Cancelled);
                }

                Err(error) => {
                    publisher.action_failed(
                        workflow,
                        serial,
                        task,
                        task_index,
                        action_index,
                        action,
                        &error.to_string(),
                    );

                    publisher.task_failed(
                        workflow,
                        serial,
                        task,
                        task_index,
                        &error.to_string(),
                    );

                    return Err(error);
                }
            }
        }

        // 3. Kiểm tra lần cuối trước khi báo thành công.
        if cancel.is_cancelled() {
            publisher.task_cancelled(
                workflow,
                serial,
                task,
                task_index,
            );

            return Err(AppError::Cancelled);
        }

        // 4. Tất cả Action hoàn thành.
        publisher.task_success(
            workflow,
            serial,
            task,
            task_index,
        );

        Ok(())
    }

    fn publish_cancelled(
            &self,
            workflow: &Workflow,
            task: &Task,
            task_index: usize,
            action_index: usize,
            action: &Action,
            publisher: &TaskEventPublisher,
        ) {
        let mut event = TaskStatusEvent::new(
            workflow.id.clone(),
            self.controller.serial(),
            task.id.clone(),
            task.name.clone(),
            task_index + 1,
            TaskStatus::Cancelled,
        );

        event.action_index = Some(action_index + 1);
        event.action_name = Some(action.name());
        event.message = Some("Task cancelled".to_string());

        publisher.publish(event);
    }

    async fn execute_action(
        &self,
        action: &Action,
        cancel: CancellationToken,
    ) -> Result<()> {
        match action {
            Action::LaunchApp { package, activity } => todo!(),
            Action::Tap { x, y } => todo!(),
            Action::TapText { text } => todo!(),
            Action::TapId { id } => todo!(),
            Action::TapClass { class_name } => todo!(),
            Action::TapContentDescription { content_desc } => todo!(),
            Action::InputText { text } => todo!(),
            Action::Swipe { x1, y1, x2, y2, duration_ms } => todo!(),
            Action::Back => todo!(),
            Action::Home => todo!(),
            Action::Screenshot => todo!(),
            Action::DumpUi => todo!(),
            Action::Wait { duration_ms } => todo!(),
            Action::WaitFor { timeout_ms, interval_ms } => todo!(),
            Action::SetVolume { value } => todo!(),
            Action::Mute => todo!(),
            Action::WifiEnable => todo!(),
            Action::WifiDisable => todo!(),
            Action::Shell { command } => todo!(),
            _ => {
                return Err(AppError::ActionFailed(
                    "Chưa triển khai Action này".to_string(),
                ));
            }
        }
    }
}