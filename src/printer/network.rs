use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    time::{Duration, timeout},
};

pub async fn send(host: &str, port: u16, data: Vec<u8>) -> Result<usize, String> {
    if host.trim().is_empty() || port == 0 {
        return Err("host y port son obligatorios".into());
    }
    let address = format!("{host}:{port}");
    let mut socket = timeout(Duration::from_secs(5), TcpStream::connect(&address))
        .await
        .map_err(|_| "tiempo de conexión agotado".to_owned())?
        .map_err(|error| format!("no se pudo conectar con {address}: {error}"))?;
    let length = data.len();
    timeout(Duration::from_secs(15), socket.write_all(&data))
        .await
        .map_err(|_| "tiempo de escritura agotado".to_owned())?
        .map_err(|error| format!("no se pudo enviar el trabajo: {error}"))?;
    socket.shutdown().await.map_err(|error| error.to_string())?;
    Ok(length)
}
