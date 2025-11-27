use sha2::{Sha256, Digest};
use crate::validate::Tx;

fn tx_type_byte(s: &str) -> u8 {
    match s {
        "transfer" => 1,
        _ => 0,
    }
}

fn token_byte(s: &str) -> u8 {
    match s {
        "GLD" => 1,
        _ => 0,
    }
}

pub fn send_tx(raw_tx: &[u8]) -> Result<Vec<u8>, String> {
    if raw_tx.len() != 269 {
        return Err(format!(
            "raw_tx must be 269 bytes, got {}",
            raw_tx.len()
        ));
    }

    let tx_hash = sha256_bytes(raw_tx);

    let mut buf = Vec::with_capacity(308);

    buf.extend_from_slice(b"FGX1");

    buf.push(1u8);

    let payload_len: u16 = 301;
    buf.extend_from_slice(&payload_len.to_be_bytes());

    buf.extend_from_slice(&tx_hash);

    buf.extend_from_slice(raw_tx);

    if buf.len() != 308 {
        return Err(format!(
            "final message must be 308 bytes, got {}",
            buf.len()
        ));
    }

    Ok(buf)
}

pub fn ask_nonce(address: &str) -> Result<Vec<u8>, String> {

    let addr_bytes = utf8_to_bytes(address);
    if addr_bytes.len() != 56 {
        return Err(format!(
            "address must be 56 bytes utf-8, got {}",
            addr_bytes.len()
        ));
    }

    let msg_type: u8 = 5;
    let payload_len: u16 = 56;

    let mut buf = Vec::with_capacity(63);

    buf.extend_from_slice(b"FGX1");

    buf.push(msg_type);

    buf.extend_from_slice(&payload_len.to_be_bytes());

    buf.extend_from_slice(&addr_bytes);

    Ok(buf)
}

pub fn ask_balance(address: &str) -> Result<Vec<u8>, String> {
    let addr_bytes = utf8_to_bytes(address);
    if addr_bytes.len() != 56 {
        return Err(format!(
            "address must be 56 bytes utf-8, got {}",
            addr_bytes.len()
        ));
    }

    let msg_type: u8 = 3;

    let payload_len: u16 = 56;

    let mut buf = Vec::with_capacity(63);

    buf.extend_from_slice(b"FGX1");

    buf.push(msg_type);

    buf.extend_from_slice(&payload_len.to_be_bytes());

    buf.extend_from_slice(&addr_bytes);

    Ok(buf)
}

pub fn make_raw_tx(tx: &Tx) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();

    push_fixed_utf8(&mut buf, &tx.domain_tag, 9, "domain_tag")?;

    push_fixed_utf8(&mut buf, &tx.chain_id, 9, "chain_id")?;

    buf.push(tx_type_byte(&tx.tx_type));

    push_fixed_utf8(&mut buf, &tx.from, 56, "from")?;

    push_fixed_utf8(&mut buf, &tx.to, 56, "to")?;

    buf.push(token_byte(&tx.token));

    buf.extend_from_slice(&tx.amount.to_be_bytes());

    buf.extend_from_slice(&tx.fee.to_be_bytes());

    buf.extend_from_slice(&tx.nonce.to_be_bytes());

    buf.extend_from_slice(&tx.timestamp.to_be_bytes());

    let pubkey_bytes = hex_to_bytes(&tx.pubkey)?;
    if pubkey_bytes.len() != 33 {
        return Err(format!(
            "pubkey must be 33 bytes, got {}",
            pubkey_bytes.len()
        ));
    }
    buf.extend_from_slice(&pubkey_bytes);

    let sig_bytes = hex_to_bytes(&tx.signature)?;
    if sig_bytes.len() != 65 {
        return Err(format!(
            "signature must be 65 bytes, got {}",
            sig_bytes.len()
        ));
    }
    buf.extend_from_slice(&sig_bytes);

    push_fixed_utf8(&mut buf, &tx.encoding, 7, "encoding")?;

    if buf.len() != 269 {
        return Err(format!("raw_tx must be 269 bytes, got {}", buf.len()));
    }

    Ok(buf)
}

fn push_fixed_utf8(buf: &mut Vec<u8>, s: &str, expected_len: usize, field: &str) -> Result<(), String> {
    let b = utf8_to_bytes(s);
    if b.len() != expected_len {
        return Err(format!(
            "{} must be {} bytes utf-8, got {}",
            field,
            expected_len,
            b.len()
        ));
    }
    buf.extend_from_slice(&b);
    Ok(())
}

fn sha256_bytes(input: &[u8]) -> [u8; 32] {
    let hash = Sha256::digest(input);

    let mut out = [0u8; 32];
    out.copy_from_slice(&hash);

    out
}

fn bytes_to_utf8(bytes: &[u8]) -> Result<String, String> {
    match String::from_utf8(bytes.to_vec()) {
        Ok(s) => Ok(s),
        Err(_) => Err("Invalid UTF-8 sequence".into()),
    }
}

fn utf8_to_bytes(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Hex string has odd length".into());
    }

    let mut bytes = Vec::with_capacity(hex.len() / 2);

    for i in (0..hex.len()).step_by(2) {
        let byte_str = &hex[i..i+2];
        let byte = u8::from_str_radix(byte_str, 16)
            .map_err(|_| format!("Invalid hex byte: {}", byte_str))?;
        bytes.push(byte);
    }

    Ok(bytes)
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);

    for b in bytes {
        hex.push_str(&format!("{:02x}", b));
    }

    hex
}