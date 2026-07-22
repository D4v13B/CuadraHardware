use std::{io::Write, time::Duration};

use serde::Serialize;
use serialport::SerialPortType;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialDevice {
    pub port_name: String,
    pub display_name: String,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub serial_number: Option<String>,
}

pub fn list_devices() -> Result<Vec<SerialDevice>, String> {
    serialport::available_ports()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|port| {
            let (display_name, vendor_id, product_id, serial_number) = match port.port_type {
                SerialPortType::UsbPort(info) => (
                    info.product
                        .unwrap_or_else(|| format!("Dispositivo USB {}", port.port_name)),
                    Some(info.vid),
                    Some(info.pid),
                    info.serial_number,
                ),
                SerialPortType::BluetoothPort => {
                    (format!("Bluetooth {}", port.port_name), None, None, None)
                }
                SerialPortType::PciPort => {
                    (format!("Puerto PCI {}", port.port_name), None, None, None)
                }
                SerialPortType::Unknown => (
                    format!("Puerto serial {}", port.port_name),
                    None,
                    None,
                    None,
                ),
            };
            Ok(SerialDevice {
                port_name: port.port_name,
                display_name,
                vendor_id,
                product_id,
                serial_number,
            })
        })
        .collect()
}

pub async fn send(port: String, baud_rate: u32, data: Vec<u8>) -> Result<usize, String> {
    tokio::task::spawn_blocking(move || {
        let mut device = serialport::new(&port, baud_rate)
            .timeout(Duration::from_secs(10))
            .open()
            .map_err(|error| format!("no se pudo abrir {port}: {error}"))?;
        device
            .write_all(&data)
            .map_err(|error| format!("error escribiendo en {port}: {error}"))?;
        device.flush().map_err(|error| error.to_string())?;
        Ok(data.len())
    })
    .await
    .map_err(|error| error.to_string())?
}
