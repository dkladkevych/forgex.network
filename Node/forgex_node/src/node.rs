use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};

use crate::mempool::{MEMPOOL, mempool_get_top, mempool_remove_by_hash};
use crate::state::{apply_block, block_info};
use crate::block::{Block, BlockHeader, BlockBody, save_block};

/// Запускает цикл ноды:
/// каждые 2 секунды берёт до 25 транзакций из мемпула,
/// делает блок, применяет его к state и сохраняет в txt.
pub async fn run_node_loop() {
    loop {
        // ждём 2 секунды между попытками
        sleep(Duration::from_secs(2)).await;

        // ---- Берём транзы из мемпула (до 25) ----
        let txs = {
            let mp = MEMPOOL.lock().unwrap();
            mempool_get_top(&mp, 25)
        };

        if txs.is_empty() {
            // ничего нет — просто пропускаем
            // println!("[NODE] mempool empty, skip");
            continue;
        }

        // ---- Определяем height и prev_hash ----
        let (prev_height, prev_hash) = block_info().unwrap_or((0, "0".repeat(64)));
        let height = prev_height + 1;

        // ---- Делаем tx_root и block_id (пока простые строки) ----
        let tx_root = make_tx_root_string(&txs);
        let timestamp_ms = current_timestamp_ms();
        let block_id = make_block_id_string(height, &tx_root, timestamp_ms);

        // ---- Собираем блок ----
        let header = BlockHeader {
            version: "0.1".to_string(),
            chain_id: "gld-dev-1".to_string(),
            height,
            prev_hash,
            timestamp_ms,
            tx_count: txs.len() as u32,
            tx_root: tx_root.clone(),
        };

        let body = BlockBody { txs: txs.clone() };

        let block = Block {
            block_id: block_id.clone(),
            header,
            body,
        };

        // ---- Применяем к state ----
        match apply_block(&block) {
            Ok(()) => {
                println!("[NODE] applied block height={}, hash={}", height, block_id);
            }
            Err(e) => {
                println!("[NODE] FAILED apply_block on height {}: {}", height, e);
                // если стейт не применился, блок дальше не сохраняем и транзы не удаляем
                continue;
            }
        }

        // ---- Сохраняем блок в txt и в память block.rs ----
        if let Err(e) = save_block(block) {
            println!("[NODE] FAILED save_block: {}", e);
        }

        // ---- Удаляем использованные транзы из мемпула ----
        {
            let mut mp = MEMPOOL.lock().unwrap();
            for tx in txs {
                mempool_remove_by_hash(&mut mp, &tx.tx_hash);
            }
        }
    }
}

/// Текущий timestamp в миллисекундах Unix.
fn current_timestamp_ms() -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards");
    now.as_millis() as i64
}

/// Простейший "tx_root": конкатенируем все tx_hash в одну строку.
/// Потом можно заменить на нормальный sha256(pipe-v1-concat).
fn make_tx_root_string(txs: &[crate::tx::ValidTxCore]) -> String {
    let mut s = String::new();
    for tx in txs {
        s.push_str(&tx.tx_hash);
    }
    s
}

/// Простейший "block_id": на основе height + кусок tx_root + timestamp.
/// Потом заменим на нормальный хеш.
fn make_block_id_string(height: u64, tx_root: &str, timestamp_ms: i64) -> String {
    let short_root = if tx_root.len() > 16 {
        &tx_root[..16]
    } else {
        tx_root
    };

    format!("blk-{}-{}-{}", height, short_root, timestamp_ms)
}
