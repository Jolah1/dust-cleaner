#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub amount_sats: u64,
    pub address: Option<String>,
}

#[allow(dead_code)]
impl Utxo {
    pub fn new(txid: &str, vout: u32, amount_sats: u64, address: Option<&str>) -> Self {
        Self {
            txid: txid.to_string(),
            vout,
            amount_sats,
            address: address.map(|a| a.to_string()),
        }
    }
}