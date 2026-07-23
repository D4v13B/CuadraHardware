use serde::{Deserialize, Serialize};
use std::{env, fs, net::IpAddr, path::PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub server: ServerConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
    pub tls_enabled: bool,
    #[serde(default = "default_http_port")]
    pub http_port: Option<u16>,
}

fn default_http_port() -> Option<u16> {
    Some(17_442)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityConfig {
    pub require_authentication: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub directory: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let data = data_dir();
        Self {
            server: ServerConfig {
                host: "127.0.0.1".parse().expect("loopback válida"),
                port: 17_443,
                tls_enabled: true,
                http_port: default_http_port(),
            },
            security: SecurityConfig {
                require_authentication: true,
            },
            logging: LoggingConfig {
                level: "info".into(),
                directory: data.join("logs"),
            },
        }
    }
}

pub fn data_dir() -> PathBuf {
    if let Some(path) = env::var_os("CUADRA_POS_AGENT_DATA_DIR") {
        return PathBuf::from(path);
    }
    #[cfg(windows)]
    {
        PathBuf::from(env::var_os("PROGRAMDATA").unwrap_or_else(|| "C:\\ProgramData".into()))
            .join("Cuadra ERP")
            .join("Cuadra POS Agent")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("data")
    }
}

pub fn load_or_create() -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    let directory = data_dir();
    fs::create_dir_all(&directory)?;
    let path = directory.join("config.json");
    if path.exists() {
        let config = parse_config(&fs::read(path)?)?;
        if !config.server.host.is_loopback() {
            return Err("server.host debe ser una dirección loopback".into());
        }
        if config.server.tls_enabled && config.server.http_port == Some(config.server.port) {
            return Err("server.httpPort debe ser diferente de server.port".into());
        }
        return Ok(config);
    }

    let config = Config::default();
    fs::write(path, serde_json::to_vec_pretty(&config)?)?;
    Ok(config)
}

fn parse_config(contents: &[u8]) -> Result<Config, serde_json::Error> {
    let contents = contents
        .strip_prefix(&[0xEF, 0xBB, 0xBF])
        .unwrap_or(contents);
    serde_json::from_slice(contents)
}

#[cfg(test)]
mod tests {
    use super::{Config, parse_config};

    #[test]
    fn existing_config_gets_default_http_port() {
        let config: Config = serde_json::from_str(
            r#"{
                "server": {"host":"127.0.0.1","port":17443,"tlsEnabled":true},
                "security": {"allowedOrigins":[],"requireAuthentication":true},
                "logging": {"level":"info","directory":"logs"}
            }"#,
        )
        .unwrap();

        assert_eq!(config.server.http_port, Some(17_442));
    }

    #[test]
    fn accepts_utf8_bom_from_windows_powershell() {
        let mut contents = vec![0xEF, 0xBB, 0xBF];
        contents.extend_from_slice(
            br#"{
                "server": {"host":"127.0.0.1","port":17442,"tlsEnabled":false},
                "security": {"requireAuthentication":true},
                "logging": {"level":"info","directory":"logs"}
            }"#,
        );

        let config = parse_config(&contents).expect("configuración UTF-8 con BOM válida");
        assert!(!config.server.tls_enabled);
        assert_eq!(config.server.port, 17_442);
    }
}
