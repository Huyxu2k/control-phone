use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    system: SystemConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SystemConfig {
    pub adb_path: String,
    pub auto_discovery: bool,
    pub device_poll_interval_ms: u64,
    pub max_concurrent_devices: usize,
    pub action_timeout_seconds: u64,
    pub retry_count: u32,
    pub worker_queue_size: usize,
}
