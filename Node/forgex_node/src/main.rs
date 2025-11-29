mod p2p;
mod model;
mod tx;
mod mempool;
mod state;
mod block;
mod node;

use anyhow::Result;
use crate::model::{Decoded, decode_message, make_tx_response, make_balance_response, make_nonce_response, make_tx_reject_response};
use crate::tx::validate_tx;
use crate::mempool::{MEMPOOL, mempool_add_tx};
use crate::state::{balance, nonce};
use crate::node::run_node_loop;

fn handle_message(msg: Vec<u8>) -> Vec<u8> {
    match decode_message(&msg) {
        // ─────────────── ТРАНЗАКЦИЯ ───────────────
        Ok(Decoded::Tx(tx)) => {
            match validate_tx(&tx) {
                Ok(valid_tx) => {
                    let mut mp = MEMPOOL.lock().unwrap();
                    println!("TX VALID: {:?}", valid_tx);

                    mempool_add_tx(&mut mp, valid_tx);
                    println!("MEMPOOL SIZE: {}", mp.len());

                    make_tx_response()
                }
                Err(e) => {
                    println!("TX INVALID: {}", e);
                    make_tx_reject_response(&e)
                }
            }
        }

        // ─────────────── ЗАПРОС БАЛАНСА ───────────────
        Ok(Decoded::AskBalance(addr, token)) => {
            println!("ASK_BALANCE: {} {}", addr, token);

            // Берём баланс из in-memory стейта
            let bal = balance(&addr, &token);
            make_balance_response(bal, &addr)
        }

        // ─────────────── ЗАПРОС NONCE ───────────────
        Ok(Decoded::AskNonce(addr)) => {
            println!("ASK_NONCE: {}", addr);

            let n = nonce(&addr);
            make_nonce_response(n, &addr)
        }

        // ─────────────── ОШИБКА ДЕКОДА ───────────────
        Err(e) => {
            println!("DECODE ERROR: {}", e);
            b"ERR".to_vec()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:5050";

    tokio::join!(
        async {
            p2p::run_p2p_server(addr, handle_message).await.unwrap();
        },
        async {
            run_node_loop().await;
        }
    );

    Ok(())
}