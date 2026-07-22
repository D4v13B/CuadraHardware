use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowsPrinter {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub port: String,
    pub status: &'static str,
    pub status_code: u32,
    pub is_default: bool,
    pub connection_type: &'static str,
}

#[cfg(windows)]
pub fn list() -> Result<Vec<WindowsPrinter>, String> {
    use std::{ptr, slice};
    use windows_sys::Win32::Graphics::Printing::{
        EnumPrintersW, GetDefaultPrinterW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL,
        PRINTER_INFO_2W, PRINTER_STATUS_ERROR, PRINTER_STATUS_OFFLINE, PRINTER_STATUS_PAPER_JAM,
        PRINTER_STATUS_PAPER_OUT, PRINTER_STATUS_PRINTING,
    };

    unsafe fn wide_ptr_to_string(value: *const u16) -> String {
        if value.is_null() {
            return String::new();
        }
        let mut length = 0;
        while unsafe { *value.add(length) } != 0 {
            length += 1;
        }
        String::from_utf16_lossy(unsafe { slice::from_raw_parts(value, length) })
    }

    let default_printer = unsafe {
        let mut length = 0_u32;
        GetDefaultPrinterW(ptr::null_mut(), &mut length);
        if length == 0 {
            String::new()
        } else {
            let mut buffer = vec![0_u16; length as usize];
            if GetDefaultPrinterW(buffer.as_mut_ptr(), &mut length) == 0 {
                String::new()
            } else {
                String::from_utf16_lossy(&buffer[..length.saturating_sub(1) as usize])
            }
        }
    };

    unsafe {
        let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
        let mut bytes_needed = 0_u32;
        let mut returned = 0_u32;
        EnumPrintersW(
            flags,
            ptr::null(),
            2,
            ptr::null_mut(),
            0,
            &mut bytes_needed,
            &mut returned,
        );
        if bytes_needed == 0 {
            return Ok(Vec::new());
        }

        let word_size = std::mem::size_of::<usize>();
        let mut storage = vec![0_usize; (bytes_needed as usize).div_ceil(word_size)];
        if EnumPrintersW(
            flags,
            ptr::null(),
            2,
            storage.as_mut_ptr().cast(),
            bytes_needed,
            &mut bytes_needed,
            &mut returned,
        ) == 0
        {
            return Err(format!(
                "No se pudieron enumerar las impresoras: {}",
                std::io::Error::last_os_error()
            ));
        }

        let entries = slice::from_raw_parts(
            storage.as_ptr().cast::<PRINTER_INFO_2W>(),
            returned as usize,
        );
        Ok(entries
            .iter()
            .map(|info| {
                let name = wide_ptr_to_string(info.pPrinterName);
                let status = if info.Status & PRINTER_STATUS_OFFLINE != 0 {
                    "offline"
                } else if info.Status
                    & (PRINTER_STATUS_ERROR | PRINTER_STATUS_PAPER_JAM | PRINTER_STATUS_PAPER_OUT)
                    != 0
                {
                    "error"
                } else if info.Status & PRINTER_STATUS_PRINTING != 0 {
                    "printing"
                } else {
                    "ready"
                };
                WindowsPrinter {
                    id: name.clone(),
                    is_default: name.eq_ignore_ascii_case(&default_printer),
                    name,
                    driver: wide_ptr_to_string(info.pDriverName),
                    port: wide_ptr_to_string(info.pPortName),
                    status,
                    status_code: info.Status,
                    connection_type: "windows",
                }
            })
            .collect())
    }
}

#[cfg(not(windows))]
pub fn list() -> Result<Vec<WindowsPrinter>, String> {
    Ok(Vec::new())
}

#[cfg(windows)]
pub async fn send(printer: String, data: Vec<u8>) -> Result<usize, String> {
    tokio::task::spawn_blocking(move || raw_print(&printer, &data))
        .await
        .map_err(|error| error.to_string())?
}

#[cfg(not(windows))]
pub async fn send(_printer: String, _data: Vec<u8>) -> Result<usize, String> {
    Err("Windows Spooler sólo está disponible en Windows".into())
}

#[cfg(windows)]
fn raw_print(printer: &str, data: &[u8]) -> Result<usize, String> {
    use std::{ffi::c_void, iter, ptr};
    use windows_sys::Win32::Graphics::Printing::{
        ClosePrinter, DOC_INFO_1W, EndDocPrinter, EndPagePrinter, OpenPrinterW, StartDocPrinterW,
        StartPagePrinter, WritePrinter,
    };

    let wide = |value: &str| {
        value
            .encode_utf16()
            .chain(iter::once(0))
            .collect::<Vec<u16>>()
    };
    let printer_name = wide(printer);
    let document_name = wide("Cuadra POS");
    let data_type = wide("RAW");
    let mut handle = ptr::null_mut();

    unsafe {
        if OpenPrinterW(printer_name.as_ptr(), &mut handle, ptr::null()) == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let document = DOC_INFO_1W {
            pDocName: document_name.as_ptr() as *mut u16,
            pOutputFile: ptr::null_mut(),
            pDatatype: data_type.as_ptr() as *mut u16,
        };
        let result = (|| {
            if StartDocPrinterW(handle, 1, &document) == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            if StartPagePrinter(handle) == 0 {
                EndDocPrinter(handle);
                return Err(std::io::Error::last_os_error().to_string());
            }
            let mut written = 0_u32;
            let ok = WritePrinter(
                handle,
                data.as_ptr() as *const c_void,
                data.len() as u32,
                &mut written,
            );
            EndPagePrinter(handle);
            EndDocPrinter(handle);
            if ok == 0 || written as usize != data.len() {
                Err(std::io::Error::last_os_error().to_string())
            } else {
                Ok(written as usize)
            }
        })();
        ClosePrinter(handle);
        result
    }
}
