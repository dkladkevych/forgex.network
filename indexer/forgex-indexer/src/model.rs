use std::convert::TryInto;
use serde::{Serialize, Deserialize};

//
// ==== ТИПЫ ДЛЯ ИНДЕКСЕРА ====
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidTxCore {
    pub tx_hash: String,
    pub tx_type: String,
    pub from: String,
    pub to: String,
    pub token: String,
    pub amount: u64,
    pub fee: u64,
    pub nonce: u64,
    pub timestamp: i64,
    pub pubkey: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: String,
    pub chain_id: String,
    pub height: u64,
    pub prev_hash: String,
    pub timestamp_ms: i64,
    pub tx_count: u32,
    pub tx_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockBody {
    pub txs: Vec<ValidTxCore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub block_id: String,
    pub header: BlockHeader,
    pub body: BlockBody,
}

const MAGIC: &[u8; 4] = b"FGX1";
const MSG_TYPE_BLOCK: u8 = 7;

//
// ==== ДЕКОДЕР БЛОКА ИЗ RAW BYTES ====
//

pub fn decode_block_raw(data: &[u8]) -> Result<Block, String> {
    // "FGX1"(4) + msg_type(1) + len(4) = 9 байт минимум
    if data.len() < 9 {
        return Err("data too short for FGX1 header".into());
    }

    // 1) magic
    if &data[0..4] != MAGIC {
        return Err("invalid magic".into());
    }

    // 2) msg_type
    let msg_type = data[4];
    if msg_type != MSG_TYPE_BLOCK {
        return Err(format!(
            "unexpected msg_type {}, expected {} (block)",
            msg_type, MSG_TYPE_BLOCK
        ));
    }

    // 3) payload_len (u32 BE)
    let payload_len = u32::from_be_bytes(data[5..9].try_into().unwrap()) as usize;
    let payload_start = 9;
    let payload_end = payload_start + payload_len;

    if payload_end > data.len() {
        return Err("payload_len out of range".into());
    }

    let payload = &data[payload_start..payload_end];
    let mut offset = 0usize;

    // ===== HEADER =====

    // block_id
    let block_id = read_str_u16(payload, &mut offset)?;

    // version
    let version = read_str_u16(payload, &mut offset)?;

    // chain_id
    let chain_id = read_str_u16(payload, &mut offset)?;

    // height
    let height = read_u64(payload, &mut offset)?;

    // prev_hash
    let prev_hash = read_str_u16(payload, &mut offset)?;

    // timestamp_ms
    let timestamp_ms = read_i64(payload, &mut offset)?;

    // tx_count
    let tx_count = read_u32(payload, &mut offset)?;

    // tx_root
    let tx_root = read_str_u16(payload, &mut offset)?;

    // ===== TXS =====

    let mut txs = Vec::with_capacity(tx_count as usize);

    for _ in 0..tx_count {
        let tx_len = read_u32(payload, &mut offset)? as usize;

        if offset + tx_len > payload.len() {
            return Err("tx_len out of range".into());
        }

        let tx_bytes = &payload[offset..offset + tx_len];
        offset += tx_len;

        let tx = decode_tx_from_block(tx_bytes)?;
        txs.push(tx);
    }

    let header = BlockHeader {
        version,
        chain_id,
        height,
        prev_hash,
        timestamp_ms,
        tx_count,
        tx_root,
    };

    let body = BlockBody { txs };

    Ok(Block {
        block_id,
        header,
        body,
    })
}

//
// ==== ДЕКОДЕР ОДНОЙ ТРАНЗАКЦИИ ====
//
// Формат такой же, как мы зашили в ноду:
//
// 32 байта  tx_hash
// 1 байт    tx_type_code
// u16 + str from
// u16 + str to
// u16 + str token
// u64       amount
// u64       fee
// u64       nonce
// i64       timestamp
// 33 байта  pubkey
// 65 байт   signature
//

fn decode_tx_from_block(buf: &[u8]) -> Result<ValidTxCore, String> {
    let mut offset = 0usize;

    if buf.len() < 32 + 1 {
        return Err("tx bytes too short".into());
    }

    // tx_hash
    let tx_hash = hex::encode(&buf[offset..offset + 32]);
    offset += 32;

    // tx_type code
    let tx_type_code = buf[offset];
    offset += 1;
    let tx_type = decode_tx_type(tx_type_code)?;

    // from
    let from = read_str_u16(buf, &mut offset)?;

    // to
    let to = read_str_u16(buf, &mut offset)?;

    // token
    let token = read_str_u16(buf, &mut offset)?;

    // amount
    let amount = read_u64(buf, &mut offset)?;

    // fee
    let fee = read_u64(buf, &mut offset)?;

    // nonce
    let nonce = read_u64(buf, &mut offset)?;

    // timestamp
    let timestamp = read_i64(buf, &mut offset)?;

    // pubkey (33 bytes)
    if offset + 33 > buf.len() {
        return Err("not enough bytes for pubkey".into());
    }
    let pubkey = hex::encode(&buf[offset..offset + 33]);
    offset += 33;

    // signature (65 bytes)
    if offset + 65 > buf.len() {
        return Err("not enough bytes for signature".into());
    }
    let signature = hex::encode(&buf[offset..offset + 65]);

    Ok(ValidTxCore {
        tx_hash,
        tx_type,
        from,
        to,
        token,
        amount,
        fee,
        nonce,
        timestamp,
        pubkey,
        signature,
    })
}

fn decode_tx_type(code: u8) -> Result<String, String> {
    match code {
        1 => Ok("transfer".into()),
        other => Err(format!("unknown tx_type code {}", other)),
    }
}

//
// ==== ХЕЛПЕРЫ ДЛЯ ЧТЕНИЯ ====
//

fn read_u16(buf: &[u8], offset: &mut usize) -> Result<u16, String> {
    if *offset + 2 > buf.len() {
        return Err("read_u16 out of bounds".into());
    }
    let v = u16::from_be_bytes(buf[*offset..*offset + 2].try_into().unwrap());
    *offset += 2;
    Ok(v)
}

fn read_u32(buf: &[u8], offset: &mut usize) -> Result<u32, String> {
    if *offset + 4 > buf.len() {
        return Err("read_u32 out of bounds".into());
    }
    let v = u32::from_be_bytes(buf[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;
    Ok(v)
}

fn read_u64(buf: &[u8], offset: &mut usize) -> Result<u64, String> {
    if *offset + 8 > buf.len() {
        return Err("read_u64 out of bounds".into());
    }
    let v = u64::from_be_bytes(buf[*offset..*offset + 8].try_into().unwrap());
    *offset += 8;
    Ok(v)
}

fn read_i64(buf: &[u8], offset: &mut usize) -> Result<i64, String> {
    if *offset + 8 > buf.len() {
        return Err("read_i64 out of bounds".into());
    }
    let v = i64::from_be_bytes(buf[*offset..*offset + 8].try_into().unwrap());
    *offset += 8;
    Ok(v)
}

fn read_str_u16(buf: &[u8], offset: &mut usize) -> Result<String, String> {
    let len = read_u16(buf, offset)? as usize;
    if *offset + len > buf.len() {
        return Err("read_str_u16 out of bounds".into());
    }

    let s = std::str::from_utf8(&buf[*offset..*offset + len])
        .map_err(|_| "invalid UTF-8 in string".to_string())?
        .to_string();

    *offset += len;
    Ok(s)
}
