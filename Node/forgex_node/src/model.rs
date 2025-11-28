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
    AskBalance(String),
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
        // TYPE 3 — ASK_BALANCE (56 bytes)
        // -------------------------------------
        3 => {
            if payload.len() != 56 {
                return Err("invalid ask_balance payload".into());
            }
            let address = String::from_utf8(payload.to_vec())
                .map_err(|_| "invalid utf8 addr".to_string())?;

            Ok(Decoded::AskBalance(address))
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
