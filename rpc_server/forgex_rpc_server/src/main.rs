mod p2p;
mod validate;
mod model;

use axum::{
    routing::{get, post},
    Json, Router, extract::Query
};

use http::Method;
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use p2p::p2p_send;
use crate::validate::Tx;
use crate::validate::parse_tx;
use model::{make_raw_tx, send_tx, ask_balance, ask_nonce, decode_p2p_response};
use sha2::{Sha256, Digest};

const IP_PORT: &str = "127.0.0.1:5050";


#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/get_info", get(get_info))
        .route("/balance", get(get_balance))
        .route("/nonce", get(get_nonce))
        .route("/broadcast_tx", post(broadcast_tx))
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("RPC server running on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

// ---------- HANDLERS ----------

async fn health_check() -> &'static str {
    "OK"
}

async fn get_info() -> Json<Value> {
    Json(json!({
        "service": "forgex_rpc",
        "version": "0.1.0",
        "status": "running"
    }))
}

async fn get_balance(Query(params): Query<Value>) -> Json<Value> {
    let address = params.get("address").and_then(|v| v.as_str()).unwrap_or("");
    let msg = ask_balance(&address).unwrap();
    let raw_res = p2p_send(IP_PORT, &msg).await.unwrap();

    let decoded = decode_p2p_response(&raw_res).unwrap();

    Json(json!({
        "address": decoded.address.unwrap(),
        "balance": decoded.balance.unwrap()
    }))
}

async fn get_nonce(Query(params): Query<Value>) -> Json<Value> {
    let address = params.get("address").and_then(|v| v.as_str()).unwrap_or("");
    let msg = ask_nonce(&address).unwrap();
    let raw_res = p2p_send(IP_PORT, &msg).await.unwrap();

    let decoded = decode_p2p_response(&raw_res).unwrap();

    Json(json!({
        "address": decoded.address.unwrap(),
        "nonce": decoded.nonce.unwrap()
    }))
}

async fn broadcast_tx(Json(body): Json<Value>) -> Json<Value> {
    let tx_opt = parse_tx(&body.to_string());

    if let Some(ref tx) = tx_opt {
        match make_raw_tx(tx) {
            Ok(raw) => {
                let tx_hash: [u8; 32] = sha256_bytes(&raw);
                let tx_hash_hex = bytes_to_hex(&tx_hash);

                let sent_tx = send_tx(&raw).unwrap();
                let raw_res = p2p_send(IP_PORT, &sent_tx).await.unwrap();

                let decoded = decode_p2p_response(&raw_res).unwrap();

                Json(json!({
                    "status": decoded.status.unwrap_or("unknown".into()),
                    "tx_hash": tx_hash_hex
                }))
            }
            Err(e) => Json(json!({
                "status": "rejected",
                "reason": format!("Raw tx build error: {}", e)
            })),
        }
    } else {
        Json(json!({
            "status": "rejected",
            "reason": "Invalid transaction"
        }))
    }
}

fn sha256_bytes(input: &[u8]) -> [u8; 32] {
    let hash = Sha256::digest(input);

    let mut out = [0u8; 32];
    out.copy_from_slice(&hash);

    out
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);

    for b in bytes {
        hex.push_str(&format!("{:02x}", b));
    }

    hex
}