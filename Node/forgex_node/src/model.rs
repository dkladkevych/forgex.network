use crate::block::Block;
use crate::tx::ValidTxCore;

const MAGIC: &[u8; 4] = b"FGX1";
const MSG_TYPE_BLOCK: u8 = 7;

// -------------------------------------------------------
// SIMPLE TX STRUCT
// -------------------------------------------------------
#[derive(Debug, Clone)]
pub struct Tx {
    pub tx_hash: String,

    pub domain_tag: String,
    pub chain_id: String,
    pub tx_type: String,
    pub from: String,
    pub to: String,
    pub token: String,
    pub amount: u64,
    pub fee: u64,
    pub nonce: u64,
    pub timestamp: u64,
    pub pubkey: String,
    pub signature: String,
    pub encoding: String,
}

// -------------------------------------------------------
// WHAT DECODER RETURNS
// -------------------------------------------------------
pub enum Decoded {
    Tx(Tx),
    AskBalance(String, String),
    AskNonce(String),
}

// -------------------------------------------------------
// PARSE FIXED UTF-8 FIELD
// -------------------------------------------------------
fn parse_fixed_utf8<'a>(
    data: &'a [u8],
    len: usize,
    field: &str,
) -> Result<(String, &'a [u8]), String> {
    if data.len() < len {
        return Err(format!("not enough bytes for {}", field));
    }
    let (chunk, rest) = data.split_at(len);
    let s = String::from_utf8(chunk.to_vec())
        .map_err(|_| format!("invalid utf-8 in {}", field))?;
    Ok((s, rest))
}

// -------------------------------------------------------
// BYTE → STRING MAPPINGS
// -------------------------------------------------------
fn tx_type_from_byte(b: u8) -> String {
    match b {
        1 => "transfer".into(),
        _ => "unknown".into(),
    }
}

fn token_from_byte(b: u8) -> String {
    match b {
        1 => "GLD".into(),
        _ => "UNKNOWN".into(),
    }
}

// -------------------------------------------------------
// HELPER (hex)
// -------------------------------------------------------
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// -------------------------------------------------------
// PARSE RAW TX (269 bytes)
// -------------------------------------------------------
pub fn parse_raw_tx(raw: &[u8]) -> Result<Tx, String> {
    use std::convert::TryInto;

    if raw.len() != 269 {
        return Err(format!("raw_tx must be 269 bytes, got {}", raw.len()));
    }

    let mut cur = raw;

    // domain_tag (9)
    let (domain_tag, rest) = parse_fixed_utf8(cur, 9, "domain_tag")?;
    cur = rest;

    // chain_id (9)
    let (chain_id, rest) = parse_fixed_utf8(cur, 9, "chain_id")?;
    cur = rest;

    // tx_type (1)
    let tx_type = tx_type_from_byte(cur[0]);
    cur = &cur[1..];

    // from (56)
    let (from, rest) = parse_fixed_utf8(cur, 56, "from")?;
    cur = rest;

    // to (56)
    let (to, rest) = parse_fixed_utf8(cur, 56, "to")?;
    cur = rest;

    // token (1)
    let token = token_from_byte(cur[0]);
    cur = &cur[1..];

    // amount (8)
    let amount = u64::from_be_bytes(cur[0..8].try_into().unwrap());
    cur = &cur[8..];

    // fee (8)
    let fee = u64::from_be_bytes(cur[0..8].try_into().unwrap());
    cur = &cur[8..];

    // nonce (8)
    let nonce = u64::from_be_bytes(cur[0..8].try_into().unwrap());
    cur = &cur[8..];

    // timestamp (8)
    let timestamp = u64::from_be_bytes(cur[0..8].try_into().unwrap());
    cur = &cur[8..];

    // pubkey (33 → hex)
    let pubkey = bytes_to_hex(&cur[0..33]);
    cur = &cur[33..];

    // signature (65 → hex)
    let signature = bytes_to_hex(&cur[0..65]);
    cur = &cur[65..];

    // encoding (7)
    let (encoding, rest) = parse_fixed_utf8(cur, 7, "encoding")?;
    cur = rest;

    if !cur.is_empty() {
        return Err("extra bytes at end of tx".into());
    }

    Ok(Tx {
        tx_hash: "".into(), // заполним позже
        domain_tag,
        chain_id,
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
        encoding,
    })
}

// -------------------------------------------------------
// MAIN DECODER FOR FGX1 PACKETS
// -------------------------------------------------------
pub fn decode_message(msg: &[u8]) -> Result<Decoded, String> {
    if msg.len() < 7 {
        return Err("too short".into());
    }

    if &msg[0..4] != b"FGX1" {
        return Err("invalid magic".into());
    }

    let msg_type = msg[4];
    let payload_len = u16::from_be_bytes([msg[5], msg[6]]) as usize;

    if msg.len() != 7 + payload_len {
        return Err("wrong length".into());
    }

    let payload = &msg[7..];

    match msg_type {
        // -------------------------------------
        // TYPE 1 — SEND_TX (tx_hash + raw_tx)
        // -------------------------------------
        1 => {
            if payload.len() != 301 {
                return Err("invalid tx payload".into());
            }

            let tx_hash_bytes = &payload[0..32];
            let tx_hash = bytes_to_hex(tx_hash_bytes);

            let raw_tx = &payload[32..];

            let mut tx = parse_raw_tx(raw_tx)?;
            tx.tx_hash = tx_hash;

            Ok(Decoded::Tx(tx))
        }

        // -------------------------------------
        // TYPE 3 — ASK_BALANCE (56 bytes addr + 3 bytes token)
        // -------------------------------------
        3 => {
            if payload.len() < 56 + 3 {
                return Err("AskBalance payload too short".into());
            }

            let addr_bytes = &payload[..56];
            let token_bytes = &payload[56..59];

            let addr = String::from_utf8(addr_bytes.to_vec())
                .map_err(|_| "invalid utf-8 in address")?;
            let token = String::from_utf8(token_bytes.to_vec())
                .map_err(|_| "invalid utf-8 in token")?;

            Ok(Decoded::AskBalance(addr, token))
        }

        // -------------------------------------
        // TYPE 5 — ASK_NONCE (56 bytes)
        // -------------------------------------
        5 => {
            if payload.len() != 56 {
                return Err("invalid ask_nonce payload".into());
            }
            let address = String::from_utf8(payload.to_vec())
                .map_err(|_| "invalid utf8 addr".to_string())?;

            Ok(Decoded::AskNonce(address))
        }

        _ => Err(format!("unsupported msg type {}", msg_type)),
    }
}

pub fn make_tx_response() -> Vec<u8> {
    let msg_type: u8 = 2;
    let payload = b"ACCEPTED";
    let payload_len = payload.len() as u16;

    let mut buf = Vec::with_capacity(4 + 1 + 2 + payload.len());

    buf.extend_from_slice(b"FGX1");
    buf.push(msg_type);
    buf.extend_from_slice(&payload_len.to_be_bytes());
    buf.extend_from_slice(payload);

    buf
}

pub fn make_tx_reject_response(reason: &str) -> Vec<u8> {
    let msg_type: u8 = 2;
    let payload = reason.as_bytes();
    let payload_len = payload.len() as u16;

    let mut buf = Vec::with_capacity(4 + 1 + 2 + payload.len());

    buf.extend_from_slice(b"FGX1");
    buf.push(msg_type);
    buf.extend_from_slice(&payload_len.to_be_bytes());
    buf.extend_from_slice(payload);

    buf
}

pub fn make_balance_response(balance: u64, address: &str) -> Vec<u8> {
    let msg_type: u8 = 4;

    let addr_bytes = address.as_bytes();
    assert!(addr_bytes.len() == 56);

    let payload_len: u16 = (8 + 56) as u16;

    let mut buf = Vec::with_capacity(4 + 1 + 2 + 64);

    buf.extend_from_slice(b"FGX1");
    buf.push(msg_type);
    buf.extend_from_slice(&payload_len.to_be_bytes());

    // balance
    buf.extend_from_slice(&balance.to_be_bytes());

    // address
    buf.extend_from_slice(addr_bytes);

    buf
}

pub fn make_nonce_response(nonce: u64, address: &str) -> Vec<u8> {
    let msg_type: u8 = 6;

    let addr_bytes = address.as_bytes();
    assert!(addr_bytes.len() == 56);

    let payload_len: u16 = (8 + 56) as u16;

    let mut buf = Vec::with_capacity(4 + 1 + 2 + 64);

    buf.extend_from_slice(b"FGX1");
    buf.push(msg_type);
    buf.extend_from_slice(&payload_len.to_be_bytes());

    buf.extend_from_slice(&nonce.to_be_bytes());
    buf.extend_from_slice(addr_bytes);

    buf
}

/// Главная функция: Block -> raw_block (FGX1|type|len|payload)
pub fn encode_block_raw(block: &Block) -> Result<Vec<u8>, String> {
    // ---------- Сначала собираем payload ----------
    let mut payload = Vec::new();

    // block_id
    write_str_u16(&mut payload, &block.block_id)?;

    // version
    write_str_u16(&mut payload, &block.header.version)?;

    // chain_id
    write_str_u16(&mut payload, &block.header.chain_id)?;

    // height (u64 BE)
    payload.extend_from_slice(&block.header.height.to_be_bytes());

    // prev_hash
    write_str_u16(&mut payload, &block.header.prev_hash)?;

    // timestamp_ms (i64 BE)
    payload.extend_from_slice(&block.header.timestamp_ms.to_be_bytes());

    // tx_count (u32 BE)
    if block.header.tx_count as usize != block.body.txs.len() {
        return Err(format!(
            "tx_count in header ({}) != body.txs.len() ({})",
            block.header.tx_count,
            block.body.txs.len()
        ));
    }
    payload.extend_from_slice(&block.header.tx_count.to_be_bytes());

    // tx_root
    write_str_u16(&mut payload, &block.header.tx_root)?;

    // ---------- Тело: транзакции ----------
    for tx in &block.body.txs {
        let tx_bytes = encode_tx_for_block(tx)?;

        let len = tx_bytes.len();
        if len > u32::MAX as usize {
            return Err("single tx too big (len > u32::MAX)".into());
        }

        // длина tx (u32 BE)
        payload.extend_from_slice(&(len as u32).to_be_bytes());
        // сами байты
        payload.extend_from_slice(&tx_bytes);
    }

    // ---------- Теперь оборачиваем в FGX1|msg_type|len|payload ----------
    let payload_len = payload.len();
    if payload_len > u32::MAX as usize {
        return Err("block payload too big (len > u32::MAX)".into());
    }

    let mut out = Vec::with_capacity(4 + 1 + 4 + payload_len);

    // MAGIC "FGX1"
    out.extend_from_slice(MAGIC);

    // msg_type = 7 (BLOCK)
    out.push(MSG_TYPE_BLOCK);

    // payload_len (u32 BE)
    out.extend_from_slice(&(payload_len as u32).to_be_bytes());

    // payload
    out.extend_from_slice(&payload);

    Ok(out)
}

//
// ===== ХЕЛПЕРЫ =====
//

/// Пишем строку как: u16 длина + UTF-8 байты
fn write_str_u16(buf: &mut Vec<u8>, s: &str) -> Result<(), String> {
    let bytes = s.as_bytes();
    let len = bytes.len();

    if len > u16::MAX as usize {
        return Err(format!(
            "string too long to encode with u16 length: {} bytes",
            len
        ));
    }

    buf.extend_from_slice(&(len as u16).to_be_bytes());
    buf.extend_from_slice(bytes);
    Ok(())
}

/// Преобразование hex-строки в байты
fn hex_to_bytes(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(format!("hex string has odd length: {}", s.len()));
    }

    let mut out = Vec::with_capacity(s.len() / 2);
    let chars: Vec<_> = s.as_bytes().to_vec();

    for i in (0..chars.len()).step_by(2) {
        let hi = hex_val(chars[i] as char)?;
        let lo = hex_val(chars[i + 1] as char)?;
        out.push((hi << 4) | lo);
    }

    Ok(out)
}

fn hex_val(c: char) -> Result<u8, String> {
    match c {
        '0'..='9' => Ok((c as u8) - b'0'),
        'a'..='f' => Ok((c as u8) - b'a' + 10),
        'A'..='F' => Ok((c as u8) - b'A' + 10),
        _ => Err(format!("invalid hex char: {}", c)),
    }
}

/// Кодирование типа транзакции в один байт
fn encode_tx_type(s: &str) -> Result<u8, String> {
    match s {
        "transfer" => Ok(1),
        // сюда потом добавишь другие типы ("mint" => 2, "stake" => 3, и т.д.)
        other => Err(format!("unknown tx_type: {}", other)),
    }
}

/// Кодирование одной уже нормализованной транзакции в бинарный формат для блока
fn encode_tx_for_block(tx: &ValidTxCore) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();

    // 1) tx_hash (32 байта)
    let tx_hash_bytes = hex_to_bytes(&tx.tx_hash)?;
    if tx_hash_bytes.len() != 32 {
        return Err(format!(
            "tx_hash must be 32 bytes, got {}",
            tx_hash_bytes.len()
        ));
    }
    buf.extend_from_slice(&tx_hash_bytes);

    // 2) tx_type (1 байт)
    let tx_type_code = encode_tx_type(&tx.tx_type)?;
    buf.push(tx_type_code);

    // 3) from, to, token (строки с префиксом длины u16)
    write_str_u16(&mut buf, &tx.from)?;
    write_str_u16(&mut buf, &tx.to)?;
    write_str_u16(&mut buf, &tx.token)?;

    // 4) amount, fee, nonce, timestamp
    buf.extend_from_slice(&tx.amount.to_be_bytes());
    buf.extend_from_slice(&tx.fee.to_be_bytes());
    buf.extend_from_slice(&tx.nonce.to_be_bytes());
    buf.extend_from_slice(&tx.timestamp.to_be_bytes());

    // 5) pubkey (33 байта)
    let pubkey_bytes = hex_to_bytes(&tx.pubkey)?;
    if pubkey_bytes.len() != 33 {
        return Err(format!(
            "pubkey must be 33 bytes, got {}",
            pubkey_bytes.len()
        ));
    }
    buf.extend_from_slice(&pubkey_bytes);

    // 6) signature (65 байт: r|s|v)
    let sig_bytes = hex_to_bytes(&tx.signature)?;
    if sig_bytes.len() != 65 {
        return Err(format!(
            "signature must be 65 bytes, got {}",
            sig_bytes.len()
        ));
    }
    buf.extend_from_slice(&sig_bytes);

    Ok(buf)
}
