mod p2p;

use anyhow::Result;

fn handle_message(msg: Vec<u8>) -> Vec<u8> {
    println!("---- New message from peer ----");
    println!("Len: {}", msg.len());
    println!("Raw bytes: {:?}", msg);

    let hex: String = msg.iter().map(|b| format!("{:02x}", b)).collect();
    println!("Hex: {}", hex);

    b"OK_FROM_NODE\n".to_vec()
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:5050";

    p2p::run_p2p_server(addr, handle_message).await
}
