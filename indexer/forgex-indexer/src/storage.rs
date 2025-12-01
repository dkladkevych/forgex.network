use std::collections::HashMap;
use std::sync::RwLock;

use once_cell::sync::Lazy;

use crate::model::{Block, BlockBody, ValidTxCore};

/// Глобальное in-memory хранилище
static STORAGE: Lazy<RwLock<InMemoryStorage>> = Lazy::new(|| {
    RwLock::new(InMemoryStorage::new())
});

#[derive(Debug)]
struct InMemoryStorage {
    /// block_id -> Block
    blocks: HashMap<String, Block>,
    /// tx_hash -> block_id
    tx_to_block: HashMap<String, String>,
}

impl InMemoryStorage {
    fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            tx_to_block: HashMap::new(),
        }
    }

    fn insert_block(&mut self, block: Block) {
        let block_id = block.block_id.clone();

        // проиндексируем все tx из блока
        for tx in &block.body.txs {
            self.tx_to_block.insert(tx.tx_hash.clone(), block_id.clone());
        }

        self.blocks.insert(block_id, block);
    }

    fn get_block(&self, block_id: &str) -> Option<Block> {
        self.blocks.get(block_id).cloned()
    }

    fn get_latest_block(&self) -> Option<Block> {
        self.blocks
            .values()
            .max_by_key(|b| b.header.height)
            .cloned()
    }

    /// Вернуть блок, в котором только ОДНА транза — та, что с этим tx_hash
    fn get_block_by_tx(&self, tx_hash: &str) -> Option<Block> {
        let block_id = self.tx_to_block.get(tx_hash)?;
        let full_block = self.blocks.get(block_id)?;

        // ищем нужную транзу внутри блока
        let tx = full_block
            .body
            .txs
            .iter()
            .find(|t| t.tx_hash == tx_hash)?
            .clone();

        // собираем новый блок с тем же хедером и id, но одной транзой
        let single_body = BlockBody { txs: vec![tx] };

        Some(Block {
            block_id: full_block.block_id.clone(),
            header: full_block.header.clone(),
            body: single_body,
        })
    }
}

// ===== публичный API =====

/// Положить блок в память
pub fn store_block(block: Block) {
    let mut s = STORAGE.write().expect("lock write");
    s.insert_block(block);
}

/// Получить блок по block_id
pub fn get_block_by_hash(block_id: &str) -> Option<Block> {
    let s = STORAGE.read().expect("lock read");
    s.get_block(block_id)
}

/// Получить последний (по height) блок
pub fn get_latest_block() -> Option<Block> {
    let s = STORAGE.read().expect("lock read");
    s.get_latest_block()
}

/// Получить блок по tx_hash, но с одной транзой в body
pub fn get_block_by_tx_hash(tx_hash: &str) -> Option<Block> {
    let s = STORAGE.read().expect("lock read");
    s.get_block_by_tx(tx_hash)
}
