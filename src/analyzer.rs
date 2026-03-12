use bitcoincore_rpc::bitcoincore_rpc_json::ListUnspentResultEntry;

pub const DUST_THRESHOLD_SATS: u64 = 1000;

pub fn is_dust(amount_sats: u64) -> bool {
    amount_sats < DUST_THRESHOLD_SATS
}

pub fn classify_utxos(
    utxos: Vec<ListUnspentResultEntry>,
) -> (Vec<ListUnspentResultEntry>, Vec<ListUnspentResultEntry>) {
    let mut dust = vec![];
    let mut clean = vec![];

    for utxo in utxos {
        if is_dust(utxo.amount.to_sat()) {
            dust.push(utxo);
        } else {
            clean.push(utxo);
        }
    }

    (dust, clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_dust() {
        assert!(is_dust(500));
        assert!(is_dust(999));
        assert!(!is_dust(1000));
        assert!(!is_dust(5000000000));
    }
}