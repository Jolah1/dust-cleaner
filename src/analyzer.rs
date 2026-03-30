use bitcoincore_rpc::bitcoincore_rpc_json::ListUnspentResultEntry;
use crate::types::Utxo;

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

#[allow(dead_code)]
pub fn classify_owned_utxos(
    utxos: Vec<Utxo>,
    threshold: u64,
) -> (Vec<Utxo>, Vec<Utxo>) {
    let mut dust = vec![];
    let mut clean = vec![];

    for utxo in utxos {
        if is_dust(utxo.amount_sats, threshold) {
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
    use crate::types::Utxo;

    fn make_utxo(txid: &str, vout: u32, amount_sats: u64) -> Utxo {
        Utxo::new(txid, vout, amount_sats, None)
    }

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

    #[test]
    fn test_classify_owned_utxos_splits_correctly() {
        let utxos = vec![
            make_utxo("txid1", 0, 500),
            make_utxo("txid2", 0, 300),
            make_utxo("txid3", 0, 5000000000),
            make_utxo("txid4", 1, 999),
            make_utxo("txid5", 0, 1000),
        ];

        let (dust, clean) = classify_owned_utxos(utxos, 1000);

        assert_eq!(dust.len(), 3);
        assert_eq!(clean.len(), 2);
        assert_eq!(dust[0].amount_sats, 500);
        assert_eq!(dust[1].amount_sats, 300);
        assert_eq!(dust[2].amount_sats, 999);
        assert_eq!(clean[0].amount_sats, 5000000000);
        assert_eq!(clean[1].amount_sats, 1000);
    }

    #[test]
    fn test_classify_owned_utxos_all_dust() {
        let utxos = vec![
            make_utxo("txid1", 0, 100),
            make_utxo("txid2", 0, 200),
        ];

        let (dust, clean) = classify_owned_utxos(utxos, 1000);

        assert_eq!(dust.len(), 2);
        assert_eq!(clean.len(), 0);
    }

    #[test]
    fn test_classify_owned_utxos_all_clean() {
        let utxos = vec![
            make_utxo("txid1", 0, 5000000000),
            make_utxo("txid2", 0, 10000),
        ];

        let (dust, clean) = classify_owned_utxos(utxos, 1000);

        assert_eq!(dust.len(), 0);
        assert_eq!(clean.len(), 2);
    }

    #[test]
    fn test_classify_owned_utxos_empty() {
        let (dust, clean) = classify_owned_utxos(vec![], 1000);
        assert_eq!(dust.len(), 0);
        assert_eq!(clean.len(), 0);
    }

    #[test]
    fn test_classify_owned_utxos_custom_threshold() {
        let utxos = vec![
            make_utxo("txid1", 0, 500),
            make_utxo("txid2", 0, 8590),
            make_utxo("txid3", 0, 5000000000),
        ];

        let (dust, clean) = classify_owned_utxos(utxos, 10000);

        assert_eq!(dust.len(), 2);
        assert_eq!(clean.len(), 1);
    }
}