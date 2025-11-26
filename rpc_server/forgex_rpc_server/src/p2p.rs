use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn p2p_send(addr: &str, data: &str) -> Result<String> {
    let mut stream = TcpStream::connect(addr).await?;

    stream.write_all(data.as_bytes()).await?;
    stream.write_all(b"\n").await?;

    let mut buffer = vec![0u8; 1024];
    let n = stream.read(&mut buffer).await?;

    let response = String::from_utf8_lossy(&buffer[..n]).to_string();
    Ok(response)
}