mod model;
mod storage;
mod p2p; // если у тебя есть p2p сервер

use axum::{
    routing::get,
    extract::Path,
    Json, Router,
};
use tower_http::cors::CorsLayer;
use http::Method;
use serde::Serialize;
use crate::model::decode_block_raw;
use crate::storage::{
    store_block,
    get_latest_block,
    get_block_by_hash,
    get_block_by_tx_hash,
};

#[derive(Serialize)]
struct LatestBlockResponse {
    block: Option<model::Block>,
}

#[derive(Serialize)]
struct BlockResponse {
    block: Option<model::Block>,
}

#[derive(Serialize)]
struct TxResponse {
    block: Option<model::Block>,
}

fn handle_p2p_msg(data: Vec<u8>) {
    match decode_block_raw(&data) {
        Ok(block) => {
            println!(
                "Decoded block: height {}, tx_count {}",
                block.header.height,
                block.header.tx_count
            );
            store_block(block);
        }
        Err(e) => {
            eprintln!("Failed to decode block: {}", e);
        }
    }
}

async fn http_latest_block() -> Json<LatestBlockResponse> {
    let block = get_latest_block();
    Json(LatestBlockResponse { block })
}

async fn http_get_block(Path(block_id): Path<String>) -> Json<BlockResponse> {
    let block = get_block_by_hash(&block_id);
    Json(BlockResponse { block })
}

async fn http_get_tx(Path(tx_hash): Path<String>) -> Json<TxResponse> {
    let block = get_block_by_tx_hash(&tx_hash);
    Json(TxResponse { block })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use tokio::net::TcpListener; 

    tokio::spawn(async {
        if let Err(e) = p2p::run_p2p_server("0.0.0.0:9000", handle_p2p_msg).await {
            eprintln!("P2P server error: {e}");
        }
    });

    // CORS: разрешим всё (для тестов ок)
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET])
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        .route("/latest_block", get(http_latest_block))
        .route("/block/:block_id", get(http_get_block))
        .route("/tx/:tx_hash", get(http_get_tx))
        .layer(cors); // <- вот это важно

    println!("HTTP RPC on http://127.0.0.2:8080");
    let listener = TcpListener::bind("127.0.0.2:8080").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
