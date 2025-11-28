use crate::model::Tx;
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Digest, Sha256};
use k256::ecdsa::{Signature, VerifyingKey};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use bech32::{self, FromBase32, ToBase32};
use k256::ecdsa::signature::Verifier;
use k256::ecdsa::signature::hazmat::PrehashVerifier;

#[derive(Debug, Clone)]
pub struct ValidTxCore {
    pub tx_hash: String,
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
}

pub fn validate_tx(tx: &Tx) -> Result<ValidTxCore, String> {
    basic_prevalidate(tx)?;
    verify_address_checksum(&tx.from)?;
    verify_address_checksum(&tx.to)?;
    verify_tx_hash(tx)?;
    verify_address(tx)?;
    verify_signature(tx)?;
    Ok(return_structured_tx(tx))
}

pub fn basic_prevalidate(tx: &Tx) -> Result<(), String> {
    // ---------------------------
    // tx_hash
    // ---------------------------
    if tx.tx_hash.is_empty() {
        return Err("tx_hash is empty".into());
    }
    if tx.tx_hash.len() != 64 {
        return Err("tx_hash must be 64 hex chars".into());
    }

    // ---------------------------
    // domain_tag
    // ---------------------------
    if tx.domain_tag.is_empty() {
        return Err("domain_tag is empty".into());
    }
    if tx.domain_tag != "GLD_TX_v1" {
        return Err("domain_tag must be GLD_TX_v1".into());
    }

    // ---------------------------
    // chain_id
    // ---------------------------
    if tx.chain_id.is_empty() {
        return Err("chain_id is empty".into());
    }
    if tx.chain_id != "gld-dev-1" {
        return Err("chain_id must be gld-dev-1".into());
    }

    // ---------------------------
    // tx_type
    // ---------------------------
    if tx.tx_type.is_empty() {
        return Err("tx_type is empty".into());
    }
    // пока только один тип
    if tx.tx_type != "transfer" {
        return Err("unsupported tx_type".into());
    }

    // ---------------------------
    // from / to
    // ---------------------------
    if tx.from.is_empty() {
        return Err("from address is empty".into());
    }
    if tx.to.is_empty() {
        return Err("to address is empty".into());
    }
    if tx.from.len() != 56 {
        return Err("from address must be 56 chars".into());
    }
    if tx.to.len() != 56 {
        return Err("to address must be 56 chars".into());
    }

    // ---------------------------
    // token
    // ---------------------------
    if tx.token.is_empty() {
        return Err("token is empty".into());
    }
    // список разрешённых токенов — пока только GLD
    if tx.token != "GLD" {
        return Err("unsupported token (only GLD allowed for now)".into());
    }

    // ---------------------------
    // amount / fee / nonce / timestamp (u64)
    // тут "не пустые" по сути значит просто что поле присутствует,
    // а оно есть по типу u64, так что проверяем базовую адекватность
    // ---------------------------
    // amount/fee можно оставить любыми >0, дальше баланс/fee логика будет отдельно
    if tx.amount == 0 {
        return Err("amount must be > 0".into());
    }
    if tx.fee == 0 {
        return Err("fee must be > 0".into());
    }
    if tx.fee > tx.amount {
        return Err("fee cannot be greater than amount".into());
    }

    // nonce просто должен быть >0 (или >=0 если хочешь разрешить 0)
    // тут оставлю >=0, это u64 и так.
    // доп. проверки nonce будут через state.

    // ---------------------------
    // timestamp: -10 минут / +5 минут в миллисекундах
    // ---------------------------
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system time before UNIX_EPOCH".to_string())?
        .as_millis() as i64;

    let tx_ts = tx.timestamp as i64;

    let min_allowed = now_ms - 10 * 60 * 1000; // -10 минут
    let max_allowed = now_ms + 5 * 60 * 1000;  // +5 минут

    if tx_ts < min_allowed {
        return Err("timestamp too old".into());
    }
    if tx_ts > max_allowed {
        return Err("timestamp too far in the future".into());
    }

    // ---------------------------
    // pubkey
    // ---------------------------
    if tx.pubkey.is_empty() {
        return Err("pubkey is empty".into());
    }
    // 33 байта в hex → 66 символов
    if tx.pubkey.len() != 66 {
        return Err("pubkey must be 33 bytes (66 hex chars)".into());
    }

    // ---------------------------
    // signature
    // ---------------------------
    if tx.signature.is_empty() {
        return Err("signature is empty".into());
    }
    // 65 байт в hex → 130 символов
    if tx.signature.len() != 130 {
        return Err("signature must be 65 bytes (130 hex chars)".into());
    }

    // ---------------------------
    // encoding
    // ---------------------------
    if tx.encoding.is_empty() {
        return Err("encoding is empty".into());
    }
    if tx.encoding != "pipe_v1" {
        return Err("encoding must be pipe_v1".into());
    }


    Ok(())
}

pub fn verify_address_checksum(addr: &str) -> Result<(), String> {
    use bech32::FromBase32;

    // 1) decode bech32
    let (hrp, data, _variant) =
        bech32::decode(addr).map_err(|_| "invalid bech32 address".to_string())?;

    // JS: decoded.hrp !== "gld2"
    if hrp != "gld2" {
        return Err("invalid address prefix (hrp must be gld2)".into());
    }

    // 2) convert 5-bit → 8-bit
    let full_bytes: Vec<u8> = <Vec<u8> as FromBase32>::from_base32(&data)
        .map_err(|_| "bech32 invalid base32 payload".to_string())?;

    // JS: if (fullBytes.length < 28) return false;
    if full_bytes.len() < 28 {
        return Err(format!("invalid raw address length: {}", full_bytes.len()));
    }

    // берём первые 28 байт как в JS: const full28 = fullBytes.slice(0, 28);
    let full28 = &full_bytes[..28];

    // JS:
    // if (full28[0] !== 'g'.charCodeAt(0) || ... '2')
    if full28[0] != b'g'
        || full28[1] != b'l'
        || full28[2] != b'd'
        || full28[3] != b'2'
    {
        return Err("address prefix bytes mismatch (expected gld2)".into());
    }

    // JS: const payload24 = full28.slice(0, 24);
    //      const checksum = full28.slice(24, 28);
    let payload24 = &full28[..24];
    let checksum = &full28[24..28];

    // JS: const hash = sha256.array(payload24);
    let hash = sha256(payload24);

    // JS: compare first 4 bytes hash vs checksum
    if &hash[..4] != checksum {
        return Err("invalid address checksum".into());
    }

    Ok(())
}

// -------------------------------------------------------------
// 1) Проверка TX_HASH
// -------------------------------------------------------------
fn verify_tx_hash(tx: &Tx) -> Result<(), String> {
    let raw = build_raw_tx_from_struct(tx)?;
    let hash = sha256(&raw);
    let hash_hex = bytes_to_hex(&hash);

    if hash_hex != tx.tx_hash {
        return Err("tx_hash mismatch".into());
    }
    Ok(())
}

// -------------------------------------------------------------
// 2) Проверка соответствия pubkey → address
// -------------------------------------------------------------
fn verify_address(tx: &Tx) -> Result<(), String> {
    use bech32::ToBase32;

    let pubkey_bytes = hex_to_bytes(&tx.pubkey)?;

    if pubkey_bytes.len() != 33 {
        return Err("pubkey must be 33 bytes".into());
    }

    // core = pubKey.slice(1)
    let core = &pubkey_bytes[1..]; // 32 байта

    // payload24[0..4] = "gld2", [4..24] = core[0..20]
    let mut payload24 = [0u8; 24];
    payload24[0] = b'g';
    payload24[1] = b'l';
    payload24[2] = b'd';
    payload24[3] = b'2';
    payload24[4..24].copy_from_slice(&core[..20]);

    // hash = sha256(payload24)
    let hash = sha256(&payload24);

    // full28 = payload24 + hash[..4]
    let mut full28 = [0u8; 28];
    full28[..24].copy_from_slice(&payload24);
    full28[24..28].copy_from_slice(&hash[..4]);

    // convertBits(8 -> 5) + bech32("gld2", ...)
    let data5 = full28.to_base32();

    let addr = bech32::encode("gld2", data5, bech32::Variant::Bech32)
        .map_err(|_| "failed to encode bech32 address".to_string())?;

    if addr != tx.from {
        return Err("address does not match pubkey".into());
    }

    Ok(())
}

// -------------------------------------------------------------
// 3) Проверка подписи
// -------------------------------------------------------------
fn verify_signature(tx: &Tx) -> Result<(), String> {
    let pubkey_bytes = hex_to_bytes(&tx.pubkey)?;
    let sig_bytes = hex_to_bytes(&tx.signature)?;

    if pubkey_bytes.len() != 33 {
        return Err("pubkey must be 33 bytes".into());
    }

    if sig_bytes.len() != 65 {
        return Err("signature must be 65 bytes".into());
    }

    // r|s (64 байта), v (1 байт, нам не нужен для проверки)
    let rs = &sig_bytes[..64];

    let signature = Signature::from_slice(rs)
        .map_err(|_| "invalid r,s signature".to_string())?;

    let verifying_key =
        VerifyingKey::from_sec1_bytes(&pubkey_bytes)
            .map_err(|_| "invalid pubkey".to_string())?;

    // === КРИТИЧЕСКИЙ МОМЕНТ ===
    // JS: hashBytes = sha256.array(from_utf8(pipe_v1_str))
    // elliptic.sign(hashHex) => подписываем ГОТОВЫЙ хэш
    let pipe_v1 = build_pipe_v1_string(tx);

    let msg_hash = sha256(pipe_v1.as_bytes()); // [u8; 32]

    // k256 по умолчанию хэширует сам, поэтому используем prehash-API:
    verifying_key
        .verify_prehash(&msg_hash, &signature)
        .map_err(|_| "signature verification failed".to_string())
}


fn build_pipe_v1_string(tx: &Tx) -> String {
    // должен соответствовать JS pipe_v1_merge(domain_tag, chain_id, tx_type, from, to, token, amount, fee, nonce, timestamp)
    format!(
        "{}{}{}{}{}{}{}{}{}{}",
        tx.domain_tag,
        tx.chain_id,
        tx.tx_type,
        tx.from,
        tx.to,
        tx.token,
        tx.amount,    // .toString() в JS == обычный десятичный формат
        tx.fee,
        tx.nonce,
        tx.timestamp,
    )
}
// -------------------------------------------------------------
// RAW_TX Сборщик (НО ОН НЕ ДОЛЖЕН УХОДИТЬ В ПРОТОКОЛ)
// Он нужен только ВНУТРИ validate_tx.
// -------------------------------------------------------------
fn build_raw_tx_from_struct(tx: &Tx) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();

    push(&mut buf, &tx.domain_tag, 9)?;
    push(&mut buf, &tx.chain_id, 9)?;
    buf.push(tx_type_byte(&tx.tx_type));
    push(&mut buf, &tx.from, 56)?;
    push(&mut buf, &tx.to, 56)?;
    buf.push(token_byte(&tx.token));

    buf.extend_from_slice(&tx.amount.to_be_bytes());
    buf.extend_from_slice(&tx.fee.to_be_bytes());
    buf.extend_from_slice(&tx.nonce.to_be_bytes());
    buf.extend_from_slice(&tx.timestamp.to_be_bytes());

    let pubkey = hex_to_bytes(&tx.pubkey)?;
    buf.extend_from_slice(&pubkey);

    let sig = hex_to_bytes(&tx.signature)?;
    buf.extend_from_slice(&sig);

    push(&mut buf, &tx.encoding, 7)?;

    Ok(buf)
}

// -------------------------------------------------------------
fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().into()
}

// small helpers
fn push(buf: &mut Vec<u8>, s: &str, len: usize) -> Result<(), String> {
    let b = s.as_bytes();
    if b.len() != len {
        return Err(format!("field must be {} bytes, got {}", len, b.len()));
    }
    buf.extend_from_slice(b);
    Ok(())
}

pub fn return_structured_tx(tx: &Tx) -> ValidTxCore {
    ValidTxCore {
        tx_hash: tx.tx_hash.clone(),
        tx_type: tx.tx_type.clone(),
        from: tx.from.clone(),
        to: tx.to.clone(),
        token: tx.token.clone(),
        amount: tx.amount,
        fee: tx.fee,
        nonce: tx.nonce,
        timestamp: tx.timestamp,
        pubkey: tx.pubkey.clone(),
        signature: tx.signature.clone(),
    }
}

fn tx_type_byte(s: &str) -> u8 { if s == "transfer" {1} else {0} }
fn token_byte(s: &str) -> u8 { if s == "GLD" {1} else {0} }

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("hex string has odd length".into());
    }

    let mut bytes = Vec::with_capacity(hex.len() / 2);

    for i in (0..hex.len()).step_by(2) {
        let byte_str = &hex[i..i + 2];
        let byte = u8::from_str_radix(byte_str, 16)
            .map_err(|_| format!("invalid hex byte: {}", byte_str))?;
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
