use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

use crate::{
    config::{Config, load_or_create},
    printer,
    security::{self, Credentials},
};

#[derive(Clone)]
struct AppState {
    config: Config,
    credentials: Credentials,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PrintResponse {
    accepted: bool,
    bytes_written: usize,
    cash_drawer_open_requested: bool,
    paper_cut_requested: bool,
}

#[derive(Serialize)]
struct KeyResponse<'a> {
    key: &'a str,
}

pub async fn run_server(
    cancellation: Option<CancellationToken>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listen_for_ctrl_c = cancellation.is_none();
    let config = load_or_create()?;
    let file_appender = std::fs::create_dir_all(&config.logging.directory)
        .map_err(|error| error.to_string())
        .and_then(|_| {
            tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("agent.log")
                .build(&config.logging.directory)
                .map_err(|error| error.to_string())
        });
    let make_filter = || {
        tracing_subscriber::EnvFilter::try_new(&config.logging.level)
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
    };
    let _log_guard = match file_appender {
        Ok(file_appender) => {
            let (writer, guard) = tracing_appender::non_blocking(file_appender);
            let _ = tracing_subscriber::fmt()
                .with_env_filter(make_filter())
                .with_writer(writer)
                .try_init();
            Some(guard)
        }
        Err(error) => {
            eprintln!(
                "No se pudo crear el archivo de log en {}: {error}. El agente continuará sin log de archivo.",
                config.logging.directory.display()
            );
            let _ = tracing_subscriber::fmt()
                .with_env_filter(make_filter())
                .with_writer(std::io::stderr)
                .try_init();
            None
        }
    };

    let state = Arc::new(AppState {
        credentials: Credentials::load_or_create()?,
        config: config.clone(),
    });
    let app = Router::new()
        .route("/", get(tester))
        .route("/tester", get(tester))
        .route("/health", get(health).options(preflight))
        .route("/v1/token", get(local_token).options(preflight))
        .route("/v1/session", get(local_token).options(preflight))
        .route("/v1/print", post(print_job).options(preflight))
        .route("/v1/printers", get(list_printers).options(preflight))
        .with_state(state);
    let cancellation = cancellation.unwrap_or_default();

    if config.server.tls_enabled {
        match load_tls_config().await {
            Ok(tls) => {
                let https_address = SocketAddr::new(config.server.host, config.server.port);
                tracing::info!(%https_address, tls = true, "iniciando Cuadra POS Agent");
                let https = serve_https(
                    https_address,
                    tls,
                    app.clone(),
                    cancellation.clone(),
                    listen_for_ctrl_c,
                );
                if let Some(http_port) = config.server.http_port {
                    let http_address = SocketAddr::new(config.server.host, http_port);
                    tracing::info!(%http_address, tls = false, "iniciando Cuadra POS Agent");
                    let http =
                        serve_http(http_address, app, cancellation.clone(), listen_for_ctrl_c);
                    tokio::pin!(https, http);
                    tokio::select! {
                        result = &mut https => {
                            if let Err(error) = result {
                                tracing::error!(%error, %https_address, "el listener HTTPS dejó de funcionar");
                            }
                            http.await?;
                        }
                        result = &mut http => {
                            if let Err(error) = result {
                                tracing::error!(%error, %http_address, "el listener HTTP dejó de funcionar");
                            }
                            https.await?;
                        }
                    }
                } else {
                    https.await?;
                }
            }
            Err(error) => {
                let http_port = config.server.http_port.unwrap_or(config.server.port);
                let http_address = SocketAddr::new(config.server.host, http_port);
                tracing::warn!(
                    %error,
                    %http_address,
                    "no se pudo preparar HTTPS; el agente continuará únicamente por HTTP"
                );
                serve_http(http_address, app, cancellation, listen_for_ctrl_c).await?;
            }
        }
    } else {
        let address = SocketAddr::new(config.server.host, config.server.port);
        tracing::info!(%address, tls = false, "iniciando Cuadra POS Agent");
        serve_http(address, app, cancellation, listen_for_ctrl_c).await?;
    }
    Ok(())
}

async fn load_tls_config()
-> Result<axum_server::tls_rustls::RustlsConfig, Box<dyn std::error::Error + Send + Sync>> {
    security::ensure_certificates()?;
    let (cert, key) = security::cert_paths();
    Ok(axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await?)
}

async fn serve_https(
    address: SocketAddr,
    tls: axum_server::tls_rustls::RustlsConfig,
    app: Router,
    cancellation: CancellationToken,
    listen_for_ctrl_c: bool,
) -> Result<(), std::io::Error> {
    let handle = axum_server::Handle::new();
    let shutdown_handle = handle.clone();
    tokio::spawn(async move {
        shutdown_signal(cancellation, listen_for_ctrl_c).await;
        shutdown_handle.graceful_shutdown(Some(Duration::from_secs(10)));
    });
    axum_server::bind_rustls(address, tls)
        .handle(handle)
        .serve(app.into_make_service())
        .await
}

async fn serve_http(
    address: SocketAddr,
    app: Router,
    cancellation: CancellationToken,
    listen_for_ctrl_c: bool,
) -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(cancellation, listen_for_ctrl_c))
        .await
}

async fn shutdown_signal(cancellation: CancellationToken, listen_for_ctrl_c: bool) {
    if !listen_for_ctrl_c {
        cancellation.cancelled().await;
        return;
    }

    tokio::select! {
        _ = cancellation.cancelled() => {},
        result = tokio::signal::ctrl_c() => {
            if let Err(error) = result {
                tracing::error!(%error, "no se pudo escuchar Ctrl+C");
                cancellation.cancelled().await;
            }
        }
    }
}

async fn health(headers: HeaderMap) -> Response {
    let origin = valid_origin(&headers);
    if headers.contains_key(header::ORIGIN) && origin.is_none() {
        return (StatusCode::FORBIDDEN, "origen no permitido").into_response();
    }
    with_cors(
        Json(HealthResponse {
            status: "ok",
            service: "cuadra-pos-agent",
            version: env!("CARGO_PKG_VERSION"),
        })
        .into_response(),
        origin.as_deref(),
    )
}

async fn tester() -> Html<&'static str> {
    Html(include_str!("../assets/tester.html"))
}

async fn local_token(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let origin = valid_origin(&headers);
    if headers.contains_key(header::ORIGIN) && origin.is_none() {
        return (StatusCode::FORBIDDEN, "origen no permitido").into_response();
    }
    let mut response = with_cors(
        Json(KeyResponse {
            key: state.credentials.token(),
        })
        .into_response(),
        origin.as_deref(),
    );
    prevent_caching(&mut response);
    response
}

fn prevent_caching(response: &mut Response) {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
}

async fn print_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<printer::PrintRequest>,
) -> Response {
    let cors_origin = valid_origin(&headers);
    let origin = match authorize(&state, &headers) {
        Ok(origin) => origin,
        Err(error) => return with_cors(error.into_response(), cors_origin.as_deref()),
    };
    match printer::print(request).await {
        Ok(outcome) => with_cors(
            (
                StatusCode::ACCEPTED,
                Json(PrintResponse {
                    accepted: true,
                    bytes_written: outcome.bytes_written,
                    cash_drawer_open_requested: outcome.cash_drawer_open_requested,
                    paper_cut_requested: outcome.paper_cut_requested,
                }),
            )
                .into_response(),
            origin.as_deref(),
        ),
        Err(message) => with_cors(
            (StatusCode::BAD_REQUEST, message).into_response(),
            origin.as_deref(),
        ),
    }
}

async fn list_printers(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let cors_origin = valid_origin(&headers);
    let origin = match authorize(&state, &headers) {
        Ok(origin) => origin,
        Err(error) => return with_cors(error.into_response(), cors_origin.as_deref()),
    };
    let windows_printers = printer::windows_spooler::list().unwrap_or_default();
    let serial_ports = printer::serial::list_devices().unwrap_or_default();
    with_cors(
        Json(serde_json::json!({
            "windowsPrinters": windows_printers,
            "serialPorts": serial_ports
        }))
        .into_response(),
        origin.as_deref(),
    )
}

async fn preflight(headers: HeaderMap) -> Response {
    let Some(_origin) = valid_origin(&headers) else {
        return (StatusCode::FORBIDDEN, "origen no permitido").into_response();
    };
    let mut response = StatusCode::NO_CONTENT.into_response();
    let output = response.headers_mut();
    output.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    output.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    output.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("authorization, content-type, x-cuadra-tester"),
    );
    output.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static("600"),
    );
    output.insert(
        "access-control-allow-private-network",
        HeaderValue::from_static("true"),
    );
    response
}

fn authorize(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<String>, (StatusCode, &'static str)> {
    if headers
        .get("x-cuadra-tester")
        .and_then(|value| value.to_str().ok())
        == Some("internal")
        && !headers.contains_key(header::ORIGIN)
    {
        return Ok(None);
    }
    if let Some(origin) = tester_origin(&state.config, headers) {
        return Ok(Some(origin));
    }
    let origin = valid_origin(headers);
    if headers.contains_key(header::ORIGIN) && origin.is_none() {
        return Err((StatusCode::FORBIDDEN, "origen no permitido"));
    }
    if state.config.security.require_authentication {
        let authorization = headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok());
        if !state.credentials.accepts_bearer(authorization) {
            return Err((StatusCode::UNAUTHORIZED, "credenciales inválidas"));
        }
    }
    Ok(origin)
}

fn valid_origin(headers: &HeaderMap) -> Option<String> {
    let origin = headers.get(header::ORIGIN)?.to_str().ok()?;
    Some(origin.to_owned())
}

fn tester_origin(config: &Config, headers: &HeaderMap) -> Option<String> {
    let origin = headers.get(header::ORIGIN)?.to_str().ok()?;
    is_tester_origin(config, origin).then(|| origin.to_owned())
}

fn is_tester_origin(config: &Config, origin: &str) -> bool {
    let is_primary = if config.server.tls_enabled {
        origin == format!("https://localhost:{}", config.server.port)
            || origin == format!("https://127.0.0.1:{}", config.server.port)
    } else {
        origin == format!("http://localhost:{}", config.server.port)
            || origin == format!("http://127.0.0.1:{}", config.server.port)
    };
    let is_additional_http = config.server.tls_enabled
        && config.server.http_port.is_some_and(|port| {
            origin == format!("http://localhost:{port}")
                || origin == format!("http://127.0.0.1:{port}")
        });
    is_primary || is_additional_http
}

fn with_cors(mut response: Response, origin: Option<&str>) -> Response {
    if origin.is_some() {
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static("*"),
        );
        response.headers_mut().insert(
            "access-control-allow-private-network",
            HeaderValue::from_static("true"),
        );
    }
    response
}

#[cfg(test)]
mod tests {
    use super::{shutdown_signal, valid_origin, with_cors};
    use axum::{
        http::{HeaderMap, HeaderValue, header},
        response::IntoResponse,
    };
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn allows_any_valid_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://otro-pos.example"),
        );

        assert_eq!(
            valid_origin(&headers).as_deref(),
            Some("https://otro-pos.example")
        );
    }

    #[test]
    fn cors_response_allows_every_origin_without_credentials() {
        let response = with_cors(().into_response(), Some("https://pos-credenciales.example"));

        assert_eq!(
            response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("*"))
        );
        assert!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
                .is_none()
        );
        assert_eq!(
            response
                .headers()
                .get("access-control-allow-private-network"),
            Some(&HeaderValue::from_static("true"))
        );
    }

    #[tokio::test]
    async fn service_shutdown_waits_for_cancellation_token() {
        let cancellation = CancellationToken::new();
        assert!(
            tokio::time::timeout(
                Duration::from_millis(25),
                shutdown_signal(cancellation.clone(), false),
            )
            .await
            .is_err()
        );

        cancellation.cancel();
        tokio::time::timeout(
            Duration::from_millis(100),
            shutdown_signal(cancellation, false),
        )
        .await
        .expect("el apagado del servicio debe responder a la cancelación");
    }
}
