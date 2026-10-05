use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::{AdbClient, Result};


pub struct DeviceController {
    adb_client: AdbClient,
    serial: String,
}

impl DeviceController {
    pub fn new(adb_client: AdbClient, serial: String) -> Self {
        Self { adb_client, serial }
    }

    pub fn serial(&self) -> &str {
        &self.serial
    }

    pub async fn shell(&self, command: &str, cancel_token: CancellationToken) -> Result<String> {
       self.adb_client.shell(&self.serial, command, cancel_token).await
    }
}