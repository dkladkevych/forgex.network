use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::tx::ValidTxCore;

// ─────────── Структуры блока ───────────

#[derive(Clone, Debug)]
pub struct BlockHeader {
    pub version: String,    // "0.1"
    pub chain_id: String,   // "gld-dev-1"
    pub height: u64,
    pub prev_hash: String,
    pub timestamp_ms: i64,
    pub tx_count: u32,
    pub tx_root: String,
}

#[derive(Clone, Debug)]
pub struct BlockBody {
    pub txs: Vec<ValidTxCore>,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub block_id: String,   // хеш блока
    pub header: BlockHeader,
    pub body: BlockBody,
}

// ─────────── Хранилище блоков в памяти ───────────

pub static BLOCKS: Lazy<Mutex<Vec<Block>>> = Lazy::new(|| Mutex::new(Vec::new()));

// ─────────── Публичные функции ───────────

/// Сохранить блок в память и записать его в txt-файл.
pub fn save_block(block: Block) -> Result<(), String> {
    {
        // кладём в in-memory список блоков
        let mut blocks = BLOCKS.lock().map_err(|_| "failed to lock BLOCKS".to_string())?;
        blocks.push(block.clone());
    }

    // пишем в файл
    write_block_to_file(&block, "blocks_log.txt")?;

    Ok(())
}

/// Получить последний сохранённый блок (если есть)
pub fn last_block() -> Option<Block> {
    let blocks = BLOCKS.lock().ok()?;
    blocks.last().cloned()
}

// ─────────── Внутренняя функция записи в файл ───────────

fn write_block_to_file(block: &Block, path: &str) -> Result<(), String> {
    // открываем файл в режиме append, создаём если нет
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("failed to open {}: {}", path, e))?;

    // формируем удобный текст для инспекции
    let mut out = String::new();

    out.push_str("============================================================\n");
    out.push_str(&format!("BLOCK ID : {}\n", block.block_id));
    out.push_str(&format!("HEIGHT   : {}\n", block.header.height));
    out.push_str(&format!("VERSION  : {}\n", block.header.version));
    out.push_str(&format!("CHAIN_ID : {}\n", block.header.chain_id));
    out.push_str(&format!("PREVHASH : {}\n", block.header.prev_hash));
    out.push_str(&format!("TIME_MS  : {}\n", block.header.timestamp_ms));
    out.push_str(&format!("TX_COUNT : {}\n", block.header.tx_count));
    out.push_str(&format!("TX_ROOT  : {}\n", block.header.tx_root));
    out.push_str("------------------------------------------------------------\n");

    for (i, tx) in block.body.txs.iter().enumerate() {
        out.push_str(&format!("TX #{}\n", i));
        out.push_str(&format!("  tx_hash   : {}\n", tx.tx_hash));
        out.push_str(&format!("  tx_type   : {}\n", tx.tx_type));
        out.push_str(&format!("  from      : {}\n", tx.from));
        out.push_str(&format!("  to        : {}\n", tx.to));
        out.push_str(&format!("  token     : {}\n", tx.token));
        out.push_str(&format!("  amount    : {}\n", tx.amount));
        out.push_str(&format!("  fee       : {}\n", tx.fee));
        out.push_str(&format!("  nonce     : {}\n", tx.nonce));
        out.push_str(&format!("  timestamp : {}\n", tx.timestamp));
        out.push_str(&format!("  pubkey    : {}\n", tx.pubkey));
        out.push_str(&format!("  signature : {}\n", tx.signature));
        out.push_str("------------------------------------------------------------\n");
    }

    out.push_str("\n");

    file.write_all(out.as_bytes())
        .map_err(|e| format!("failed to write block to {}: {}", path, e))?;

    Ok(())
}
