use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn p2p_send(addr: &str, data: &str) -> Result<String> {
    // подключаемся
    let mut stream = TcpStream::connect(addr).await?;

    // отправляем сообщение
    stream.write_all(data.as_bytes()).await?;
    stream.write_all(b"\n").await?;

    // читаем ответ
    let mut buffer = vec![0u8; 1024];
    let n = stream.read(&mut buffer).await?;

    let response = String::from_utf8_lossy(&buffer[..n]).to_string();
    Ok(response)
}