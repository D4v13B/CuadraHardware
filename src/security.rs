use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair, KeyUsagePurpose,
    SanType,
};
use std::{
    fs, io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    path::PathBuf,
};

use crate::config::data_dir;

#[derive(Clone)]
pub struct Credentials {
    token: String,
}

impl Credentials {
    pub fn load_or_create() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let path = data_dir().join("agent-token");
        if path.exists() {
            let token = fs::read_to_string(path)?.trim().to_owned();
            if token.len() < 32 {
                return Err("agent-token es inválido o demasiado corto".into());
            }
            return Ok(Self { token });
        }

        let mut bytes = [0_u8; 48];
        getrandom::fill(&mut bytes)
            .map_err(|error| format!("no se pudo generar entropía segura: {error}"))?;
        let token = URL_SAFE_NO_PAD.encode(bytes);
        write_secret(&path, token.as_bytes())?;
        Ok(Self { token })
    }

    pub fn accepts_bearer(&self, value: Option<&str>) -> bool {
        let Some(candidate) = value.and_then(|v| v.strip_prefix("Bearer ")) else {
            return false;
        };
        constant_time_eq(candidate.as_bytes(), self.token.as_bytes())
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |diff, (a, b)| diff | (a ^ b))
        == 0
}

pub fn cert_paths() -> (PathBuf, PathBuf) {
    let certs = data_dir().join("certs");
    (certs.join("localhost.crt"), certs.join("localhost.key"))
}

pub fn ensure_certificates() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let certs = data_dir().join("certs");
    fs::create_dir_all(&certs)?;
    let root_path = certs.join("root-ca.crt");
    let (server_cert_path, server_key_path) = cert_paths();
    if root_path.exists() && server_cert_path.exists() && server_key_path.exists() {
        return Ok(root_path);
    }

    let mut ca_params = CertificateParams::default();
    let mut ca_name = DistinguishedName::new();
    ca_name.push(DnType::CommonName, "Cuadra POS Local Root CA");
    ca_name.push(DnType::OrganizationName, "Cuadra ERP");
    ca_params.distinguished_name = ca_name;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    let ca_key = KeyPair::generate()?;
    let ca_cert = ca_params.self_signed(&ca_key)?;

    let mut leaf_params = CertificateParams::new(vec!["localhost".to_owned()])?;
    leaf_params
        .subject_alt_names
        .push(SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)));
    leaf_params
        .subject_alt_names
        .push(SanType::IpAddress(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    let mut leaf_name = DistinguishedName::new();
    leaf_name.push(DnType::CommonName, "localhost");
    leaf_name.push(DnType::OrganizationName, "Cuadra ERP");
    leaf_params.distinguished_name = leaf_name;
    leaf_params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    let leaf_key = KeyPair::generate()?;
    let leaf_cert = leaf_params.signed_by(&leaf_key, &ca_cert, &ca_key)?;

    fs::write(&root_path, ca_cert.pem())?;
    fs::write(&server_cert_path, leaf_cert.pem())?;
    write_secret(&server_key_path, leaf_key.serialize_pem().as_bytes())?;
    Ok(root_path)
}

pub fn install_local_ca() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = ensure_certificates()?;
    #[cfg(windows)]
    {
        let status = std::process::Command::new("certutil.exe")
            .args(["-f", "-addstore", "Root"])
            .arg(&root)
            .status()?;
        if !status.success() {
            return Err(format!("certutil terminó con {status}").into());
        }
    }
    #[cfg(not(windows))]
    tracing::warn!(path = %root.display(), "instale manualmente la CA local en este sistema");
    Ok(())
}

fn write_secret(path: &std::path::Path, contents: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = options.open(path)?;
    file.write_all(contents)?;
    file.sync_all()?;
    restrict_to_service(path)
}

#[cfg(windows)]
fn restrict_to_service(path: &std::path::Path) -> io::Result<()> {
    let user = match (std::env::var("USERDOMAIN"), std::env::var("USERNAME")) {
        (Ok(domain), Ok(name)) => format!("{domain}\\{name}:F"),
        (_, Ok(name)) => format!("{name}:F"),
        _ => "SYSTEM:F".to_owned(),
    };
    let status = std::process::Command::new("icacls.exe")
        .arg(path)
        .args(["/inheritance:r", "/grant:r", "SYSTEM:F", "Administrators:F"])
        .arg(user)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("icacls no pudo proteger el secreto"))
    }
}

#[cfg(not(windows))]
fn restrict_to_service(path: &std::path::Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn compares_tokens() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"other!"));
        assert!(!constant_time_eq(b"short", b"longer"));
    }
}
