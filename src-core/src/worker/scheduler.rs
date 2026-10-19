use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{AppError, Result, Workflow};

use super::manager::WorkerManager;

#[derive(Clone)]
pub struct SchedulerHandle {
    sender: mpsc::Sender<Workflow>,
}

impl SchedulerHandle {
    pub fn new(sender: mpsc::Sender<Workflow>) -> Self {
        Self { sender }
    }

    pub async fn submit(&self, workflow: Workflow) -> Result<()> {
        self.sender
            .send(workflow)
            .await
            .map_err(|_| AppError::Cancelled)
    }
}

pub struct Scheduler {
    receiver: mpsc::Receiver<Workflow>,
    worker_manager: Arc<WorkerManager>,
    cancel: CancellationToken,
}

impl Scheduler {
    pub fn new(
        receiver: mpsc::Receiver<Workflow>,
        worker_manager: Arc<WorkerManager>,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            receiver,
            worker_manager,
            cancel,
        }
    }

    pub async fn run(mut self) {
        loop {
            let workflow = tokio::select! {
                _ = self.cancel.cancelled() => {
                    tracing::info!("Scheduler cancelled");
                    break;
                }

                workflow = self.receiver.recv() => {
                    match workflow {
                        Some(workflow) => workflow,
                        None => break,
                    }
                }
            };

            // Mỗi lần chỉ dispatch một Workflow.
            // Workflow tiếp theo đợi Workflow hiện tại kết thúc.
            let result = tokio::select! {
                _ = self.cancel.cancelled() => {
                    Err(AppError::Cancelled)
                }

                result = self.dispatch_workflow(workflow) => result,
            };

            if let Err(error) = result {
                tracing::error!(
                    error = %error,
                    "Workflow dispatch failed"
                );
            }
        }
    }

    async fn dispatch_workflow(&self, workflow: Workflow) -> Result<()> {
        let workflow = Arc::new(workflow);

        // Lấy tất cả Worker đang Idle và phù hợp với selector.
        let workers = self.worker_manager.find_available_workers(&workflow).await;

        if workers.is_empty() {
            return Err(AppError::ActionFailed(
                "Không có điện thoại nào sẵn sàng chạy Workflow".to_string(),
            ));
        }

        tracing::info!(
            workflow_id = %workflow.id,
            device_count = workers.len(),
            "Dispatching workflow to devices"
        );

        let mut completions = Vec::with_capacity(workers.len());

        // Gửi cùng một Workflow cho từng điện thoại.
        for worker in workers {
            if self.cancel.is_cancelled() {
                return Err(AppError::Cancelled);
            }

            match worker.submit(Arc::clone(&workflow)).await {
                Ok(completion) => completions.push((worker.serial.clone(), completion)),
                Err(error) => {
                    tracing::warn!(
                        serial = %worker.serial,
                        error = %error,
                        "Failed to submit workflow to device"
                    );
                }
            }
        }

        if completions.is_empty() {
            return Err(AppError::ActionFailed(
                "Không gửi được Workflow đến điện thoại nào".to_string(),
            ));
        }

        // Không dùng JoinHandle: oneshot nhận kết quả Workflow
        // từ mỗi Worker. Scheduler chỉ chuyển sang Workflow kế
        // tiếp khi tất cả kết quả đã được nhận.
        let mut first_error: Option<AppError> = None;

        for (serial, completion) in completions {
            let result = tokio::select! {
                _ = self.cancel.cancelled() => {
                    return Err(AppError::Cancelled);
                }

                result = completion => result,
            };

            match result {
                Ok(Ok(())) => {
                    tracing::info!(
                        serial = %serial,
                        workflow_id = %workflow.id,
                        "Workflow completed"
                    );
                }

                Ok(Err(error)) => {
                    tracing::warn!(
                        serial = %serial,
                        workflow_id = %workflow.id,
                        error = %error,
                        "Workflow failed on device"
                    );

                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }

                Err(_) => {
                    tracing::warn!(
                        serial = %serial,
                        "Worker dropped completion channel"
                    );

                    if first_error.is_none() {
                        first_error = Some(AppError::DeviceOffline(serial));
                    }
                }
            }
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}
