use std::collections::HashMap;
use std::sync::Mutex;
use crate::block::Block;
use once_cell::sync::Lazy;

use crate::tx::ValidTxCore;

/// Казна
pub const TREASURY_ADDR: &str =
    "gld21vakxgv57d5snlk3t06emkcu7uyk5snmu5xtsp7sjtasx69tqawy";

/// 1_000_000_000 GLD в минимальных юнитах (если у тебя 6 знаков после запятой)
pub const TREASURY_INITIAL_GLD: u64 = 1_000_000_000_000_000; // 1e9 * 1e6


// ─────────────────────── ВНУТРЕННЕЕ СОСТОЯНИЕ ───────────────────────

#[derive(Debug)]
pub struct BlockMeta {
    pub height: u64,
    pub hash: String,
}

#[derive(Debug)]
pub struct ChainState {
    /// Балансы по (address, token)
    pub balances: HashMap<(String, String), u64>,
    /// Nonce по адресу (один nonce на адрес)
    pub nonces: HashMap<String, u64>,
    /// Последний применённый блок
    pub latest_block: Option<BlockMeta>,
}

/// Глобальное состояние ноды (in-memory)
pub static STATE: Lazy<Mutex<ChainState>> = Lazy::new(|| {
    let mut balances = HashMap::new();

    // Казне сразу даём 1e9 GLD
    balances.insert(
        (TREASURY_ADDR.to_string(), "GLD".to_string()),
        TREASURY_INITIAL_GLD,
    );

    Mutex::new(ChainState {
        balances,
        nonces: HashMap::new(),
        latest_block: None,
    })
});

// ─────────────────────── 1) apply_block ───────────────────────

/// Применить блок к состоянию.
/// Списывает amount+fee с from, зачисляет amount на to,
/// fee отправляет в казну, обновляет nonce и latest_block.
pub fn apply_block(block: &Block) -> Result<(), String> {
    let mut state = STATE.lock().unwrap();

    // Можно на всякий случай проверить tx_count
    if block.header.tx_count as usize != block.body.txs.len() {
        println!(
            "[STATE] WARN: header.tx_count={} but body.txs.len()={}",
            block.header.tx_count,
            block.body.txs.len()
        );
    }

    for tx in &block.body.txs {
        // Пока обрабатываем только transfer
        if tx.tx_type != "transfer" {
            println!("[STATE] skip tx_type {} for now", tx.tx_type);
            continue;
        }

        let from = tx.from.clone();
        let to = tx.to.clone();
        let token = tx.token.clone();

        let total_debit = tx.amount
            .checked_add(tx.fee)
            .ok_or_else(|| "overflow on amount+fee".to_string())?;

        // ── Списываем с отправителя ──
        let from_key = (from.clone(), token.clone());
        let from_balance = state.balances.entry(from_key.clone()).or_insert(0);

        if *from_balance < total_debit {
            return Err(format!(
                "insufficient balance for {}, need {}, have {}",
                from, total_debit, *from_balance
            ));
        }

        *from_balance -= total_debit;

        // ── Зачисляем получателю amount ──
        let to_key = (to.clone(), token.clone());
        let to_balance = state.balances.entry(to_key).or_insert(0);
        *to_balance = to_balance
            .checked_add(tx.amount)
            .ok_or_else(|| "overflow on receiver balance".to_string())?;

        // ── Комиссию отправляем в казну ──
        let treasury_key = (TREASURY_ADDR.to_string(), token.clone());
        let treasury_balance = state.balances.entry(treasury_key).or_insert(0);
        *treasury_balance = treasury_balance
            .checked_add(tx.fee)
            .ok_or_else(|| "overflow on treasury balance".to_string())?;

        // ── Обновляем nonce отправителя ──
        let nonce_entry = state.nonces.entry(from.clone()).or_insert(0);
        if tx.nonce > *nonce_entry {
            *nonce_entry = tx.nonce;
        }
    }

    // Сохраняем информацию о последнем блоке
    state.latest_block = Some(BlockMeta {
        height: block.header.height,
        hash: block.block_id.clone(),
    });

    Ok(())
}

// ─────────────────────── 2) balance(addr, token) ───────────────────────

pub fn balance(addr: &str, token: &str) -> u64 {
    let state = STATE.lock().unwrap();
    *state
        .balances
        .get(&(addr.to_string(), token.to_string()))
        .unwrap_or(&0)
}

// ─────────────────────── 3) nonce(addr) ───────────────────────

pub fn nonce(addr: &str) -> u64 {
    let state = STATE.lock().unwrap();
    *state.nonces.get(addr).unwrap_or(&0)
}

// ─────────────────────── 4) block_info() ───────────────────────

/// Вернуть (height, hash) последнего блока.
/// Если блоков ещё нет — None.
pub fn block_info() -> Option<(u64, String)> {
    let state = STATE.lock().unwrap();
    state
        .latest_block
        .as_ref()
        .map(|b| (b.height, b.hash.clone()))
}
