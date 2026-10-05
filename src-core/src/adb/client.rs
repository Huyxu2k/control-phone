use std::process::Stdio;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::{AdbDeviceInfo, parse_devices_output};
use crate::{AppError, Result};

#[derive(Clone)]
pub struct AdbClient {
    adb_path: String,
}

impl AdbClient {
    pub fn new(adb_path: impl Into<String>) -> Self {
        Self {
            adb_path: adb_path.into(),
        }
    }

    pub fn adb_path(&self) -> &str {
        &self.adb_path
    }

    pub async fn devices(&self) -> Result<Vec<AdbDeviceInfo>> {
        let output = Command::new(&self.adb_path).arg("devices").output().await?;

        if !output.status.success() {
            return Err(AppError::AdbCommand(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(parse_devices_output(&stdout))
    }

    pub async fn shell(
        &self,
        serial: &str,
        command: &str,
        cancel: CancellationToken,
    ) -> Result<String> {
        self.shell_args(serial, vec![command], cancel).await
    }

    pub async fn shell_args(
        &self,
        serial: &str,
        args: Vec<&str>,
        cancel: CancellationToken,
    ) -> Result<String> {
        let mut command = Command::new(&self.adb_path)
            .arg("-s")
            .arg(serial)
            .arg("shell")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        tokio::select! {
            result = command.wait_with_output() => {
                let output = result?;

                if !output.status.success() {
                    let error = String::from_utf8_lossy(&output.stderr).to_string();
                    if Self::is_device_error(&error) {
                        return Err(AppError::AdbCommand(
                            String::from_utf8_lossy(&output.stderr).to_string(),
                        ));
                    }
                    return Err(
                        AppError::AdbCommand(
                            error.to_string()
                        )
                    );
                }
                Ok(String::from_utf8_lossy(
                    &output.stdout
                ).to_string())
            }
            _ = cancel.cancelled() => {
                Err(AppError::Cancelled)
            }
        }
    }

    pub async fn exec(
        &self,
        serial: &str,
        args: Vec<&str>,
        cancel: CancellationToken,
    ) -> Result<String> {
        let mut command = Command::new(&self.adb_path)
            .arg("-s")
            .arg(serial)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        tokio::select! {
            result = command.wait_with_output() => {
                let output = result?;

                if !output.status.success() {

                    return Err(
                        AppError::AdbCommand(
                           String::from_utf8_lossy(&output.stderr).to_string()
                        )
                    );
                }
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            }
            _ = cancel.cancelled() => {
                Err(AppError::Cancelled)
            }
        }
    }

    pub async fn is_device_online(&self, serial: &str) -> bool {
        match self.devices().await {
            Ok(devices) => devices.iter().any(|device| device.serial == serial && device.state == "device"),
            Err(_) => false,
        }
    }

    fn is_device_error(error: &str) -> bool {
       error.contains("device offline")
            || error.contains("device not found")
            || error.contains("no devices")
            || error.contains("closed")
            || error.contains("transport")
    }
}
