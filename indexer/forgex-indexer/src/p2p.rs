use anyhow::Result;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

/// handler: функция, которая принимает сырые байты сообщения
/// НИЧЕГО не возвращает, просто обрабатывает (логика в main)
pub async fn run_p2p_server(
    addr: &str,
    handler: fn(Vec<u8>),
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

async fn handle_connection(mut stream: TcpStream, handler: fn(Vec<u8>)) {
    let mut buf = vec![0u8; 4096];

    loop {
        match stream.read(&mut buf).await {
            Ok(0) => {
                println!("Peer disconnected");
                return; // соединение закрыто
            }
            Ok(n) => {
                let msg = buf[..n].to_vec();
                println!("Received {} bytes", n);

                // просто передаём данные в handler (который ты объявишь в main)
                handler(msg);
                // НИЧЕГО не пишем в stream
            }
            Err(e) => {
                eprintln!("Socket read error: {}", e);
                return;
            }
        }
    }
}