use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::RwLock;

use crate::{AdbClient, Result, device, parse_devices_output};
use super::device::{AndroidDevice, DeviceState};


pub struct DeviceManager {
    adb_client: AdbClient,
    devices: Arc<RwLock<HashMap<String, AndroidDevice>>>,
}

impl DeviceManager {
    pub fn new(adb_client: AdbClient) -> Self {
        Self {
            adb_client,
            devices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn refresh_devices(&self) -> Result<Vec<AndroidDevice>> {
        let devices = self.adb_client.devices().await?;
        let mut devices_map = self.devices.write().await;

        for device in devices {
            let state = if device.state == "device" {
                DeviceState::Online
            } else {
                DeviceState::Offline
            };

            devices_map.entry(device.serial.clone())
                .and_modify(|d| d.state = state.clone())
                .or_insert(AndroidDevice {
                    serial: device.serial,
                    state,
                });
        }  
       
        let current_serials =devices_map
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>();
        let adb_serials =
            self.adb_client
                .devices()
                .await?
                .into_iter()
                .map(|x| x.serial)
                .collect::<std::collections::HashSet<_>>();
        for serial in current_serials {
            if !adb_serials.contains(&serial) {
                if let Some(device) =
                    devices_map.get_mut(&serial)
                {
                    device.state =
                        DeviceState::Offline;
                }
            }
        }

        Ok(devices_map.values().cloned().collect())
    }

    pub async fn get_devices(&self) -> Vec<AndroidDevice> {
        let devices_map = self.devices.read().await;
        devices_map.values().cloned().collect()
    }


}