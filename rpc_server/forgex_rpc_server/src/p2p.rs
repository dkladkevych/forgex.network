use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn p2p_send(addr: &str, data: &[u8]) -> Result<Vec<u8>> {
    let mut stream = TcpStream::connect(addr).await?;

    stream.write_all(data).await?;

    stream.write_all(&[b'\n']).await?;

    let mut buffer = vec![0u8; 4096];
    let n = stream.read(&mut buffer).await?;

    Ok(buffer[..n].to_vec())
}