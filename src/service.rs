#[cfg(windows)]
use std::{ffi::OsString, sync::mpsc, time::Duration};
#[cfg(windows)]
use tokio_util::sync::CancellationToken;
#[cfg(windows)]
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

#[cfg(windows)]
const SERVICE_NAME: &str = "CuadraPosAgent";

#[cfg(windows)]
define_windows_service!(ffi_service_main, service_main);

#[cfg(windows)]
pub fn start_service_dispatcher() -> Result<(), windows_service::Error> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

#[cfg(windows)]
fn service_main(_arguments: Vec<OsString>) {
    if let Err(error) = run_service() {
        eprintln!("El servicio terminó con error: {error}");
    }
}

#[cfg(windows)]
fn run_service() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let event_handler = move |control_event| match control_event {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = stop_tx.send(());
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    let status = service_control_handler::register(SERVICE_NAME, event_handler)?;
    status.set_service_status(service_status(
        ServiceState::StartPending,
        ServiceControlAccept::empty(),
        1,
        Duration::from_secs(10),
    ))?;

    let cancellation = CancellationToken::new();
    let server_token = cancellation.clone();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    status.set_service_status(service_status(
        ServiceState::Running,
        ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        0,
        Duration::default(),
    ))?;

    let server_result = runtime.block_on(async move {
        let mut server = tokio::spawn(crate::server::run_server(Some(server_token)));
        let stop = tokio::task::spawn_blocking(move || stop_rx.recv());
        tokio::select! {
            result = &mut server => result,
            _ = stop => {
                cancellation.cancel();
                server.await
            }
        }
    });
    status.set_service_status(service_status(
        ServiceState::Stopped,
        ServiceControlAccept::empty(),
        0,
        Duration::default(),
    ))?;
    match server_result {
        Ok(result) => result,
        Err(error) => Err(error.into()),
    }
}

#[cfg(windows)]
fn service_status(
    state: ServiceState,
    accepted: ServiceControlAccept,
    checkpoint: u32,
    wait_hint: Duration,
) -> ServiceStatus {
    ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: accepted,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint,
        wait_hint,
        process_id: None,
    }
}
