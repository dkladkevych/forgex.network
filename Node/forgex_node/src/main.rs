mod p2p;
mod model;
mod tx;

use anyhow::Result;
use crate::model::{Decoded, Tx, decode_message, make_tx_response, make_balance_response, make_nonce_response, make_tx_reject_response};
use crate::tx::validate_tx;

fn handle_message(msg: Vec<u8>) -> Vec<u8> {
    match decode_message(&msg) {
        Ok(Decoded::Tx(tx)) => {
            println!("TX RECEIVED:");
            println!("{:#?}", tx);
            match validate_tx(&tx) {
                Ok(valid_tx) => {
                    println!("TX VALID {:?}", valid_tx);
                    make_tx_response()
                }
                Err(e) => {
                    println!("TX INVALID: {}", e);
                    make_tx_reject_response(&e)
                }
            }
        }

        Ok(Decoded::AskBalance(addr)) => {
            println!("ASK_BALANCE: {}", addr);

            // временно баланс=123456
            let balance = 123456u64;
            make_balance_response(balance, &addr)
        }

        Ok(Decoded::AskNonce(addr)) => {
            println!("ASK_NONCE: {}", addr);

            let nonce = 42u64;
            make_nonce_response(nonce, &addr)
        }

        Err(e) => {
            println!("DECODE ERROR: {}", e);
            b"ERR".to_vec()
        }
    }
}



#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:5050";

    p2p::run_p2p_server(addr, handle_message).await
}
