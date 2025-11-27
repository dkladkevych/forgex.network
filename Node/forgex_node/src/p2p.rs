use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// handler: функция, которая принимает сырые байты сообщения и возвращает ответ в байтах
pub async fn run_p2p_server(
    addr: &str,
    handler: fn(Vec<u8>) -> Vec<u8>,
) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("P2P node listening on {}", addr);

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        println!("Incoming connection from {}", peer_addr);

        // на каждое соединение — отдельная таска
        tokio::spawn(handle_connection(socket, handler));
    }
}

async fn handle_connection(mut stream: TcpStream, handler: fn(Vec<u8>) -> Vec<u8>) {
    let mut buf = vec![0u8; 4096];

    match stream.read(&mut buf).await {
        Ok(0) => {
            // клиент закрыл соединение
            println!("Empty read, client closed connection");
        }
        Ok(n) => {
            buf.truncate(n);
            println!("Received {} bytes from peer", n);

            // тут просто передаём в main
            let response = handler(buf);

            // отправляем ответ
            if let Err(e) = stream.write_all(&response).await {
                eprintln!("Failed to send response: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to read from socket: {}", e);
        }
    }
}