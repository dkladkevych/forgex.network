use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::tx::ValidTxCore;

pub type Mempool = HashMap<String, ValidTxCore>;

pub static MEMPOOL: Lazy<Mutex<Mempool>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn mempool_add_tx(m: &mut Mempool, tx: ValidTxCore) {

    if m.contains_key(&tx.tx_hash) {
        return;
    }

    if let Some((existing_hash, existing_fee)) = m
        .iter()
        .find(|(_, existing)| existing.from == tx.from && existing.nonce == tx.nonce)
        .map(|(h, existing)| (h.clone(), existing.fee))
    {
        if tx.fee <= existing_fee {

            return;
        } else {

            m.remove(&existing_hash);
        }
    }

    m.insert(tx.tx_hash.clone(), tx);
}

pub fn mempool_get_top(m: &Mempool, count: usize) -> Vec<ValidTxCore> {

    let mut txs: Vec<ValidTxCore> = m.values().cloned().collect();

    txs.sort_by(|a, b| b.fee.cmp(&a.fee));

    let real_count = std::cmp::min(count, txs.len());
    txs.into_iter().take(real_count).collect()
}

pub fn mempool_remove_by_hash(m: &mut Mempool, tx_hash: &str) -> Option<ValidTxCore> {
    m.remove(tx_hash)
}