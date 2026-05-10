use bluer::Session;
use std::sync::{Arc, Mutex, Weak};

#[derive(Debug, Clone)]
pub struct BluetoothBattery {
    pub name: String,
    pub percentage: u8,
}

#[derive(Clone)]
pub struct BluetoothBatteryMonitor {
    batteries: Arc<Mutex<Vec<BluetoothBattery>>>,
}

impl BluetoothBatteryMonitor {
    pub fn new() -> Self {
        Self {
            batteries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn start(&self) {
        let batteries = Arc::downgrade(&self.batteries);

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Failed to create tokio runtime: {}", e);
                    return;
                }
            };

            rt.block_on(async move {
                let session = match Session::new().await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to create bluetooth session: {}", e);
                        return;
                    }
                };

                loop {
                    if let Some(batteries_ref) = batteries.upgrade() {
                        if let Err(e) = Self::refresh_batteries(&session, &batteries_ref).await {
                            tracing::error!("Error refreshing bluetooth batteries: {}", e);
                        }
                    } else {
                        break;
                    }

                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            });
        });
    }

    async fn refresh_batteries(
        session: &Session,
        batteries: &Arc<Mutex<Vec<BluetoothBattery>>>,
    ) -> Result<(), bluer::Error> {
        let adapter_names = session.adapter_names().await?;

        let mut all_batteries = Vec::new();

        for adapter_name in adapter_names {
            let adapter = match session.adapter(&adapter_name) {
                Ok(a) => a,
                Err(_) => continue,
            };

            let device_addresses = match adapter.device_addresses().await {
                Ok(addrs) => addrs,
                Err(_) => continue,
            };

            for addr in device_addresses {
                let device = match adapter.device(addr) {
                    Ok(d) => d,
                    Err(_) => continue,
                };

                if let Ok(true) = device.is_connected().await {
                    if let Some(percentage) = device.battery_percentage().await.ok().flatten() {
                        let name = match device.alias().await {
                            Ok(alias) => alias,
                            Err(_) => addr.to_string(),
                        };

                        all_batteries.push(BluetoothBattery { name, percentage });
                    }
                }
            }
        }

        let mut batteries_guard = batteries.lock().unwrap();
        tracing::info!("Bluetooth batteries updated: {:?}", all_batteries);
        *batteries_guard = all_batteries;

        Ok(())
    }

    pub fn get_batteries(&self) -> Vec<BluetoothBattery> {
        self.batteries.lock().unwrap().clone()
    }
}
