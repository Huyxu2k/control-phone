use std::{collections::HashMap, sync::Arc};

use tokio::sync::{RwLock, Semaphore, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::{AppError, DeviceController, Result, TaskEventPublisher, Workflow, ActionExecutor};

use super::worker::{DeviceWorker, WorkerJob};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    Idle,
    Busy,
    Offline,
    Error,
    Stopped,
}

#[derive(Clone)]
pub struct WorkerHandle {
    pub serial: String,
    sender: mpsc::Sender<WorkerJob>,
    state: Arc<RwLock<WorkerState>>,
}

impl WorkerHandle {
    pub async fn state(&self) -> WorkerState {
        *self.state.read().await
    }

    pub async fn submit(&self, workflow: Arc<Workflow>) -> Result<oneshot::Receiver<Result<()>>> {
        let (completion_tx, completion_rx) = oneshot::channel();

        self.sender
            .send(WorkerJob {
                workflow,
                completion: completion_tx,
            })
            .await
            .map_err(|_| AppError::DeviceOffline(self.serial.clone()))?;

        Ok(completion_rx)
    }

    pub async fn set_state(&self, state: WorkerState) {
        *self.state.write().await = state;
    }
}

pub struct WorkerManager {
    workers: RwLock<HashMap<String, WorkerHandle>>,
    queue_size: usize,
    concurrency: Arc<Semaphore>,
    root_cancel: CancellationToken,
    publisher: TaskEventPublisher,
}

impl WorkerManager {
    pub fn new(
        max_concurrent_devices: usize,
        queue_size: usize,
        root_cancel: CancellationToken,
        publisher: TaskEventPublisher,
    ) -> Self {
        Self {
            workers: RwLock::new(HashMap::new()),
            queue_size,
            concurrency: Arc::new(Semaphore::new(max_concurrent_devices.max(1))),
            root_cancel,
            publisher,
        }
    }

    pub async fn add_device(&self, serial: String, controller: DeviceController) {
        let mut workers = self.workers.write().await;

        if workers.contains_key(&serial) {
            return;
        }

        let (sender, receiver) = mpsc::channel::<WorkerJob>(self.queue_size);

        let state = Arc::new(RwLock::new(WorkerState::Idle));

        let executor = ActionExecutor::new(controller);

        let worker = DeviceWorker::new(
            serial.clone(),
            receiver,
            Arc::clone(&state),
            self.root_cancel.child_token(),
            Arc::clone(&self.concurrency),
            self.publisher.clone(),
            executor,
        );

        // Cố ý không lưu JoinHandle theo yêu cầu.
        // Worker được đánh thức bằng CancellationToken.
        tokio::spawn(worker.run());

        workers.insert(
            serial.clone(),
            WorkerHandle {
                serial,
                sender,
                state,
            },
        );
    }

    pub async fn get(&self, serial: &str) -> Option<WorkerHandle> {
        self.workers.read().await.get(serial).cloned()
    }

    pub async fn list(&self) -> Vec<WorkerHandle> {
        self.workers.read().await.values().cloned().collect()
    }

    pub async fn find_available_workers(&self, workflow: &Workflow) -> Vec<WorkerHandle> {
        let workers = self.workers.read().await;
        let mut available = Vec::new();

        for worker in workers.values() {
            if !workflow.selector.matches(&worker.serial) {
                continue;
            }

            if worker.state().await == WorkerState::Idle {
                available.push(worker.clone());
            }
        }

        available
    }

    pub async fn mark_offline(&self, serial: &str) {
        if let Some(worker) = self.get(serial).await {
            worker.set_state(WorkerState::Offline).await;
        }
    }

    pub async fn mark_idle(&self, serial: &str) {
        if let Some(worker) = self.get(serial).await {
            // Không đưa Worker trở lại Idle nếu toàn ứng dụng
            // đang shutdown.
            if !self.root_cancel.is_cancelled() {
                worker.set_state(WorkerState::Idle).await;
            }
        }
    }

    pub async fn busy_count(&self) -> usize {
        let workers = self.list().await;
        let mut count = 0;

        for worker in workers {
            if worker.state().await == WorkerState::Busy {
                count += 1;
            }
        }

        count
    }
}
