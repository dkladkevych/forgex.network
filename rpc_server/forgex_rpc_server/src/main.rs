mod p2p;
mod validate;

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
use validate::{Tx, parse_tx};


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

    println!("Balance request for: {}", address);

    Json(json!({
        "address": address,
        "balance": 43
    }))
}

async fn get_nonce(Query(params): Query<Value>) -> Json<Value> {
    let address = params.get("address").and_then(|v| v.as_str()).unwrap_or("");

    println!("Nonce request for: {}", address);

    Json(json!({
        "address": address,
        "nonce": 1
    }))
}
    // let resp = p2p_send("127.0.0.1:5050", &body.to_string()).await.unwrap();

async fn broadcast_tx(Json(body): Json<Value>) -> Json<Value> {
    let tx_opt = parse_tx(&body.to_string());

    match tx_opt {
        Some(tx) => {
            println!("Parsed tx: {:?}", tx);
            Json(json!({
                "status": "accepted",
                "tx_hash": "0x123"
            }))
        }
        None => {
            println!("Invalid tx received: {}", body);
            Json(json!({
                "status": "rejected",
                "reason": "Invalid transaction"
            }))
        }
    }
    
}