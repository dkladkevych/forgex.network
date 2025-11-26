use serde::{Serialize, Deserialize};
use serde_json::{Value, Map};
use bech32::FromBase32;
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tx {
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

fn get_u64_from_mixed(obj: &Map<String, Value>, key: &str) -> Option<u64> {
    let val = obj.get(key)?;
    if let Some(n) = val.as_u64() {
        return Some(n);
    }
    if let Some(s) = val.as_str() {
        let s_trim = s.trim();
        if s_trim.is_empty() {
            return None;
        }
        if let Ok(n) = s_trim.parse::<u64>() {
            return Some(n);
        }
    }
    None
}

pub fn parse_tx(json_str: &str) -> Option<Tx> {
    let v: Value = serde_json::from_str(json_str).ok()?;
    let obj = v.as_object()?;

    let mut tx = Tx {
        domain_tag: String::new(),
        chain_id: String::new(),
        tx_type: String::new(),
        from: String::new(),
        to: String::new(),
        token: String::new(),
        amount: 0,
        fee: 0,
        nonce: 0,
        timestamp: 0,
        pubkey: String::new(),
        signature: String::new(),
        encoding: String::new(),
    };


    // domain_tag
    {
        let s = obj.get("domain_tag")?.as_str()?;
        if s.is_empty() { return None; }
        tx.domain_tag = s.to_string();
    }

    // chain_id
    {
        let s = obj.get("chain_id")?.as_str()?;
        if s.is_empty() { return None; }
        tx.chain_id = s.to_string();
    }

    // tx_type
    {
        let s = obj.get("tx_type")?.as_str()?;
        if s.is_empty() { return None; }
        tx.tx_type = s.to_string();
    }

    // from
    {
        let s = obj.get("from")?.as_str()?;
        if s.is_empty() { return None; }
        if !validate_address(s) { return None; }
        tx.from = s.to_string();
    }

    // to
    {
        let s = obj.get("to")?.as_str()?;
        if s.is_empty() { return None; }
        if !validate_address(s) { return None; }
        tx.to = s.to_string();
    }

    // token
    {
        let s = obj.get("token")?.as_str()?;
        if s.is_empty() { return None; }
        tx.token = s.to_string();
    }

    // pubkey (JSON: "pub_key")
    {
        let s = obj.get("pub_key")?.as_str()?;
        if s.is_empty() { return None; }
        tx.pubkey = s.to_string();
    }

    // signature (JSON: "sig")
    {
        let s = obj.get("sig")?.as_str()?;
        if s.is_empty() { return None; }
        tx.signature = s.to_string();
    }

    // encoding
    {
        let s = obj.get("encoding")?.as_str()?;
        if s.is_empty() { return None; }
        tx.encoding = s.to_string();
    }

    // amount
    {
        let n = get_u64_from_mixed(obj, "amount")?;
        if n == 0 { return None; }
        tx.amount = n;
    }

    // fee
    {
        let n = get_u64_from_mixed(obj, "fee")?;
        if n == 0 { return None; }
        tx.fee = n;
    }

    // nonce
    {
        let n = get_u64_from_mixed(obj, "nonce")?;
        tx.nonce = n;
    }

    // timestamp: "timestamp" or "timestamp_ms"
    {
        if let Some(n) = get_u64_from_mixed(obj, "timestamp") {
            tx.timestamp = n;
        } else if let Some(ms) = get_u64_from_mixed(obj, "timestamp_ms") {
            tx.timestamp = ms;
        } else {
            return None;
        }
    }

    Some(tx)
}

pub fn validate_address(addr: &str) -> bool {
    let (hrp, data5, _variant) = match bech32::decode(addr) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if hrp != "gld2" {
        return false;
    }

    let data8: Vec<u8> = match Vec::<u8>::from_base32(&data5) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if data8.len() != 28 {
        return false;
    }

    if &data8[0..4] != b"gld2" {
        return false;
    }

    let payload24 = &data8[0..24];
    let checksum4 = &data8[24..28];

    let mut hasher = Sha256::new();
    hasher.update(payload24);
    let hash = hasher.finalize();

    if &hash[0..4] != checksum4 {
        return false;
    }

    true
}
