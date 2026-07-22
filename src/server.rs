use axum::{
    Json, Router,
    extract::{ConnectInfo, State},
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
struct TokenResponse<'a> {
    token: &'a str,
}

pub async fn run_server(
    cancellation: Option<CancellationToken>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = load_or_create()?;
    std::fs::create_dir_all(&config.logging.directory)?;
    let file_appender = tracing_appender::rolling::daily(&config.logging.directory, "agent.log");
    let (writer, _guard) = tracing_appender::non_blocking(file_appender);
    let filter = tracing_subscriber::EnvFilter::try_new(&config.logging.level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .try_init();

    let state = Arc::new(AppState {
        credentials: Credentials::load_or_create()?,
        config: config.clone(),
    });
    let app = Router::new()
        .route("/", get(tester))
        .route("/tester", get(tester))
        .route("/health", get(health).options(preflight))
        .route("/v1/token", get(local_token).options(preflight))
        .route("/v1/print", post(print_job).options(preflight))
        .route("/v1/printers", get(list_printers).options(preflight))
        .with_state(state);
    let address = SocketAddr::new(config.server.host, config.server.port);
    let cancellation = cancellation.unwrap_or_default();
    tracing::info!(%address, tls = config.server.tls_enabled, "iniciando Cuadra POS Agent");

    if config.server.tls_enabled {
        security::ensure_certificates()?;
        let (cert, key) = security::cert_paths();
        let tls = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await?;
        let https = serve_https(address, tls, app.clone(), cancellation.clone());
        if let Some(http_port) = config.server.http_port {
            let http_address = SocketAddr::new(config.server.host, http_port);
            tracing::info!(%http_address, tls = false, "iniciando Cuadra POS Agent");
            tokio::try_join!(https, serve_http(http_address, app, cancellation.clone()))?;
        } else {
            https.await?;
        }
    } else {
        serve_http(address, app, cancellation).await?;
    }
    Ok(())
}

async fn serve_https(
    address: SocketAddr,
    tls: axum_server::tls_rustls::RustlsConfig,
    app: Router,
    cancellation: CancellationToken,
) -> Result<(), std::io::Error> {
    let handle = axum_server::Handle::new();
    let shutdown_handle = handle.clone();
    tokio::spawn(async move {
        shutdown_signal(cancellation).await;
        shutdown_handle.graceful_shutdown(Some(Duration::from_secs(10)));
    });
    axum_server::bind_rustls(address, tls)
        .handle(handle)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
}

async fn serve_http(
    address: SocketAddr,
    app: Router,
    cancellation: CancellationToken,
) -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(cancellation))
    .await
}

async fn shutdown_signal(cancellation: CancellationToken) {
    tokio::select! {
        _ = cancellation.cancelled() => {},
        result = tokio::signal::ctrl_c() => {
            if let Err(error) = result {
                tracing::error!(%error, "no se pudo escuchar Ctrl+C");
            }
        }
    }
}

async fn health(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let origin = valid_origin(&state.config, &headers);
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

async fn local_token(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if !is_local_peer(peer) {
        return (
            StatusCode::FORBIDDEN,
            "el token sólo está disponible localmente",
        )
            .into_response();
    }
    let origin = valid_origin(&state.config, &headers);
    if headers.contains_key(header::ORIGIN) && origin.is_none() {
        return (StatusCode::FORBIDDEN, "origen no permitido").into_response();
    }
    let mut response = with_cors(
        Json(TokenResponse {
            token: state.credentials.token(),
        })
        .into_response(),
        origin.as_deref(),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response
}

fn is_local_peer(peer: SocketAddr) -> bool {
    peer.ip().is_loopback()
}

async fn print_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<printer::PrintRequest>,
) -> Response {
    let origin = match authorize(&state, &headers) {
        Ok(origin) => origin,
        Err(error) => return error.into_response(),
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
    let origin = match authorize(&state, &headers) {
        Ok(origin) => origin,
        Err(error) => return error.into_response(),
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

async fn preflight(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let Some(origin) = valid_origin(&state.config, &headers) else {
        return (StatusCode::FORBIDDEN, "origen no permitido").into_response();
    };
    let mut response = StatusCode::NO_CONTENT.into_response();
    let output = response.headers_mut();
    output.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_str(&origin).expect("origin validado"),
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
    if headers
        .get("access-control-request-private-network")
        .and_then(|value| value.to_str().ok())
        == Some("true")
    {
        output.insert(
            "access-control-allow-private-network",
            HeaderValue::from_static("true"),
        );
    }
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
    let origin = valid_origin(&state.config, headers);
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

fn valid_origin(config: &Config, headers: &HeaderMap) -> Option<String> {
    let origin = headers.get(header::ORIGIN)?.to_str().ok()?;
    if is_tester_origin(config, origin) {
        return Some(origin.to_owned());
    }
    config
        .security
        .allowed_origins
        .iter()
        .any(|allowed| allowed == "*" || allowed == origin)
        .then(|| origin.to_owned())
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
    if let Some(origin) = origin.and_then(|value| HeaderValue::from_str(value).ok()) {
        response
            .headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        response
            .headers_mut()
            .insert(header::VARY, HeaderValue::from_static("Origin"));
    }
    response
}

#[cfg(test)]
mod tests {
    use super::{is_local_peer, valid_origin};
    use crate::config::Config;
    use axum::http::{HeaderMap, HeaderValue, header};

    #[test]
    fn wildcard_allows_any_valid_origin() {
        let mut config = Config::default();
        config.security.allowed_origins = vec!["*".into()];
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://otro-pos.example"),
        );

        assert_eq!(
            valid_origin(&config, &headers).as_deref(),
            Some("https://otro-pos.example")
        );
    }

    #[test]
    fn token_peer_must_be_loopback() {
        assert!(is_local_peer("127.0.0.1:50000".parse().unwrap()));
        assert!(is_local_peer("[::1]:50000".parse().unwrap()));
        assert!(!is_local_peer("192.168.1.25:50000".parse().unwrap()));
    }
}
