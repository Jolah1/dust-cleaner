use bitcoincore_rpc::bitcoincore_rpc_json::ListUnspentResultEntry;



pub fn is_dust(amount_sats: u64, threshold: u64) -> bool {
    amount_sats < threshold
}

pub fn classify_utxos(
    utxos: Vec<ListUnspentResultEntry>,
    threshold: u64,
) -> (Vec<ListUnspentResultEntry>, Vec<ListUnspentResultEntry>) {
    let mut dust = vec![];
    let mut clean = vec![];

    for utxo in utxos {
        if is_dust(utxo.amount.to_sat(), threshold) {
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
    fn test_is_dust_default_threshold() {
        assert!(is_dust(500, 1000));
        assert!(is_dust(999, 1000));
        assert!(!is_dust(1000, 1000));
        assert!(!is_dust(5000000000, 1000));
    }

    #[test]
    fn test_is_dust_custom_threshold() {
        assert!(is_dust(500, 2000));
        assert!(is_dust(1999, 2000));
        assert!(!is_dust(2000, 2000));
        assert!(!is_dust(5000, 2000));
    }

    #[test]
    fn test_is_dust_zero_threshold() {
        assert!(!is_dust(0, 0));
        assert!(!is_dust(1, 0));
    }
}