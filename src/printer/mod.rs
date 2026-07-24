mod network;
pub mod serial;
pub mod windows_spooler;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "transport", rename_all = "camelCase")]
pub enum PrintRequest {
    Network {
        host: String,
        port: u16,
        #[serde(rename = "dataBase64", alias = "data_base64")]
        data_base64: String,
        #[serde(default)]
        cash: bool,
        #[serde(default)]
        cut: bool,
    },
    Serial {
        port: String,
        #[serde(rename = "baudRate", alias = "baud_rate")]
        baud_rate: u32,
        #[serde(rename = "dataBase64", alias = "data_base64")]
        data_base64: String,
        #[serde(default)]
        cash: bool,
        #[serde(default)]
        cut: bool,
    },
    WindowsSpooler {
        printer: String,
        #[serde(rename = "dataBase64", alias = "data_base64")]
        data_base64: String,
        #[serde(default)]
        cash: bool,
        #[serde(default)]
        cut: bool,
    },
}

pub struct PrintOutcome {
    pub bytes_written: usize,
    pub cash_drawer_open_requested: bool,
    pub paper_cut_requested: bool,
}

pub async fn print(request: PrintRequest) -> Result<PrintOutcome, String> {
    let (bytes_written, cash_drawer_open_requested, paper_cut_requested) = match request {
        PrintRequest::Network {
            host,
            port,
            data_base64,
            cash,
            cut,
        } => (
            network::send(&host, port, prepare_payload(&data_base64, cash, cut)?).await?,
            cash,
            cut,
        ),
        PrintRequest::Serial {
            port,
            baud_rate,
            data_base64,
            cash,
            cut,
        } => (
            serial::send(port, baud_rate, prepare_payload(&data_base64, cash, cut)?).await?,
            cash,
            cut,
        ),
        PrintRequest::WindowsSpooler {
            printer,
            data_base64,
            cash,
            cut,
        } => {
            let mut bytes_written =
                windows_spooler::send(printer.clone(), prepare_payload(&data_base64, cash, false)?)
                    .await?;
            if cut {
                // Some Windows ESC/POS drivers accept cutting only when it is
                // sent as its own RAW document, which is also how the tester's
                // dedicated cut action operates.
                bytes_written += windows_spooler::send(printer, cut_job_payload()).await?;
            }
            (bytes_written, cash, cut)
        }
    };
    Ok(PrintOutcome {
        bytes_written,
        cash_drawer_open_requested,
        paper_cut_requested,
    })
}

fn prepare_payload(value: &str, cash: bool, cut: bool) -> Result<Vec<u8>, String> {
    use base64::Engine;
    let mut bytes = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| "dataBase64 no contiene Base64 válido".to_owned())?;
    const MAX_JOB_BYTES: usize = 8 * 1024 * 1024;
    if bytes.is_empty() || bytes.len() > MAX_JOB_BYTES {
        return Err("el trabajo debe contener entre 1 byte y 8 MiB".to_owned());
    }
    if cash {
        bytes.extend_from_slice(&[0x1b, 0x70, 0x00, 25, 250]);
    }
    if cut {
        // Feed the receipt past the cutter before using the basic partial-cut
        // variant supported by older and clone ESC/POS printers such as XP-80.
        bytes.extend_from_slice(&[0x0a, 0x0a, 0x0a, 0x1d, 0x56, 0x01]);
    }
    Ok(bytes)
}

fn cut_job_payload() -> Vec<u8> {
    vec![0x1b, 0x40, 0x0a, 0x0a, 0x0a, 0x1d, 0x56, 0x01]
}

#[cfg(test)]
mod tests {
    use super::{PrintRequest, cut_job_payload, prepare_payload};

    #[test]
    fn deserializes_documented_windows_spooler_request() {
        let request: PrintRequest = serde_json::from_str(
            r#"{
                "transport": "windowsSpooler",
                "dataBase64": "SG9sYQ==",
                "cash": false,
                "cut": true,
                "printer": "RONGTA 80mm Series Printer"
            }"#,
        )
        .unwrap();

        assert!(matches!(
            request,
            PrintRequest::WindowsSpooler {
                printer,
                data_base64,
                cash: false,
                cut: true,
            } if printer == "RONGTA 80mm Series Printer" && data_base64 == "SG9sYQ=="
        ));
    }

    #[test]
    fn deserializes_documented_serial_field_names() {
        let request: PrintRequest = serde_json::from_str(
            r#"{
                "transport": "serial",
                "port": "COM3",
                "baudRate": 9600,
                "dataBase64": "SG9sYQ=="
            }"#,
        )
        .unwrap();

        assert!(matches!(
            request,
            PrintRequest::Serial {
                port,
                baud_rate: 9600,
                data_base64,
                cash: false,
                cut: false,
            } if port == "COM3" && data_base64 == "SG9sYQ=="
        ));
    }

    #[test]
    fn does_not_open_drawer_when_cash_is_false() {
        assert_eq!(prepare_payload("SG9sYQ==", false, false).unwrap(), b"Hola");
    }

    #[test]
    fn appends_drawer_pulse_when_cash_is_true() {
        assert_eq!(
            prepare_payload("SG9sYQ==", true, false).unwrap(),
            [b"Hola".as_slice(), &[0x1b, 0x70, 0x00, 25, 250]].concat()
        );
    }

    #[test]
    fn appends_cut_when_cut_is_true() {
        assert_eq!(
            prepare_payload("SG9sYQ==", false, true).unwrap(),
            [b"Hola".as_slice(), &[0x0a, 0x0a, 0x0a, 0x1d, 0x56, 0x01]].concat()
        );
    }

    #[test]
    fn appends_drawer_pulse_before_cut() {
        assert_eq!(
            prepare_payload("SG9sYQ==", true, true).unwrap(),
            [
                b"Hola".as_slice(),
                &[0x1b, 0x70, 0x00, 25, 250],
                &[0x0a, 0x0a, 0x0a, 0x1d, 0x56, 0x01],
            ]
            .concat()
        );
    }

    #[test]
    fn creates_standalone_windows_cut_job_like_the_tester() {
        assert_eq!(
            cut_job_payload(),
            [0x1b, 0x40, 0x0a, 0x0a, 0x0a, 0x1d, 0x56, 0x01]
        );
    }
}
