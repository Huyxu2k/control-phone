use std::sync::Arc;
use tokio::sync::{
    RwLock, Semaphore, mpsc, oneshot
};

use tokio_util::sync::CancellationToken;

use crate::{ActionExecutor, AppError, DeviceController, Result, Task, TaskEventPublisher, WorkerState, Workflow};

pub struct WorkerJob {
    pub workflow: Arc<Workflow>,
    pub completion: oneshot::Sender<Result<()>>
}

pub struct DeviceWorker {
    serial: String,
    receiver: mpsc::Receiver<WorkerJob>,
    state: Arc<tokio::sync::RwLock<WorkerState>>,
    root_cancel: CancellationToken,
    concurrency: Arc<Semaphore>,
    publisher: TaskEventPublisher,
    executor: ActionExecutor,
}

impl DeviceWorker {
    pub fn new(
        serial: String,
        receiver: mpsc::Receiver<WorkerJob>,
        state: Arc<RwLock<WorkerState>>,
        root_cancel: CancellationToken,
        concurrency: Arc<Semaphore>,
        publisher: TaskEventPublisher,
        executor: ActionExecutor,
    ) -> Self {
        Self {
            serial,
            receiver,
            state,
            root_cancel,
            concurrency,
            publisher,
            executor,
        }
    }

    pub async fn run(mut self) {
        loop {
            let job = tokio::select! {
                _ = self.root_cancel.cancelled() => {
                    break;
                }

                job = self.receiver.recv() => {
                    match job {
                        Some(job) => job,
                        None => break,
                    }
                }
            };

            // Mỗi Workflow chỉ chiếm một slot trong giới hạn
            // số điện thoại chạy đồng thời.
            let result = tokio::select! {
                _ = self.root_cancel.cancelled() => {
                    Err(AppError::Cancelled)
                }

                permit = self.concurrency.clone().acquire_owned() => {
                    match permit {
                        Ok(_permit) => {
                            *self.state.write().await = WorkerState::Busy;

                            let workflow_cancel =
                                self.root_cancel.child_token();

                            let result = tokio::select! {
                                _ = self.root_cancel.cancelled() => {
                                    Err(AppError::Cancelled)
                                }

                                result = self.execute_workflow(
                                    &job.workflow,
                                    workflow_cancel,
                                ) => result,
                            };

                            result
                        }

                        Err(_) => Err(AppError::Cancelled),
                    }
                }
            };

            // Báo kết quả về Scheduler. Không cần JoinHandle
            // để theo dõi việc hoàn thành của từng Workflow.
            let _ = job.completion.send(result);

            let mut state = self.state.write().await;

            if *state != WorkerState::Offline
                && *state != WorkerState::Stopped
            {
                *state = if self.root_cancel.is_cancelled() {
                    WorkerState::Stopped
                } else {
                    WorkerState::Idle
                };
            }

            if self.root_cancel.is_cancelled() {
                break;
            }
        }

        *self.state.write().await = WorkerState::Stopped;
    }

    async fn execute_workflow(
        &self,
        workflow: &Workflow,
        cancel: CancellationToken,
    ) -> Result<()> {
        for (task_index, task) in workflow.tasks.iter().enumerate() {
            if cancel.is_cancelled() {
                return Err(AppError::Cancelled);
            }

            self.execute_task(
                workflow,
                task,
                task_index,
                cancel.child_token(),
            )
            .await?;
        }

        Ok(())
    }

    async fn execute_task(
        &self,
        workflow: &Workflow,
        task: &Task,
        task_index: usize,
        cancel: CancellationToken,
    ) -> Result<()> {
        if cancel.is_cancelled() {
            return Err(AppError::Cancelled);
        }

        // ActionExecutor cần kiểm tra cancel trong lúc thực thi
        // Action, đồng thời phát event trạng thái Task/Action.
        self.executor
            .execute_task(
                workflow,
                task,
                task_index,
                &self.publisher,
                cancel,
            )
            .await
    }

    pub fn serial(&self) -> &str {
        &self.serial
    }
}