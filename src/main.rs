mod config;
mod printer;
mod security;
mod server;
mod service;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|arg| arg == "--install-ca") {
        if let Err(error) = security::install_local_ca() {
            eprintln!("No se pudo preparar el certificado local: {error}");
            std::process::exit(1);
        }
        return;
    }

    if args.iter().any(|arg| arg == "--service") {
        #[cfg(windows)]
        if let Err(error) = service::start_service_dispatcher() {
            eprintln!("No se pudo iniciar el servicio: {error}");
            std::process::exit(1);
        }

        #[cfg(not(windows))]
        {
            eprintln!("El modo servicio sólo está disponible en Windows.");
            std::process::exit(1);
        }
        return;
    }

    if !args.iter().any(|arg| arg == "--no-browser") {
        open_tester_in_browser();
    }

    if let Err(error) = run_console() {
        eprintln!("Error del agente: {error}");
        std::process::exit(1);
    }
}

#[cfg(windows)]
fn open_tester_in_browser() {
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(1200));
        let _ = std::process::Command::new("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg("https://localhost:17443/tester")
            .spawn();
    });
}

#[cfg(not(windows))]
fn open_tester_in_browser() {}

fn run_console() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(server::run_server(None))
}
