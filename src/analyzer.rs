use bitcoincore_rpc::bitcoincore_rpc_json::ListUnspentResultEntry;
use crate::types::Utxo;

/// Dust thresholds per script type based on fee cost to spend
pub const DUST_P2PKH: u64 = 546;
pub const DUST_P2WPKH: u64 = 294;
pub const DUST_P2TR: u64 = 294;
pub const DUST_P2SH: u64 = 540;
pub const DUST_DEFAULT: u64 = 546;

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptType {
    P2PKH,
    P2WPKH,
    P2TR,
    P2SH,
    Unknown,
}

impl ScriptType {
    pub fn dust_threshold(&self) -> u64 {
        match self {
            ScriptType::P2PKH => DUST_P2PKH,
            ScriptType::P2WPKH => DUST_P2WPKH,
            ScriptType::P2TR => DUST_P2TR,
            ScriptType::P2SH => DUST_P2SH,
            ScriptType::Unknown => DUST_DEFAULT,
        }
    }
}

/// Detect script type from address prefix
pub fn detect_script_type(address: &str) -> ScriptType {
    if address.starts_with("1") {
        ScriptType::P2PKH
    } else if address.starts_with("3") {
        ScriptType::P2SH
    } else if address.starts_with("bc1q") || address.starts_with("tb1q") || address.starts_with("bcrt1q") {
        ScriptType::P2WPKH
    } else if address.starts_with("bc1p") || address.starts_with("tb1p") || address.starts_with("bcrt1p") {
        ScriptType::P2TR
    } else {
        ScriptType::Unknown
    }
}

/// Check if a UTXO is dust using per-script-type threshold
/// If user provides a custom threshold, that overrides the per-type threshold
pub fn is_dust(amount_sats: u64, threshold: u64) -> bool {
    amount_sats < threshold
}

pub fn is_dust_smart(amount_sats: u64, address: Option<&str>, user_threshold: Option<u64>) -> bool {
    let threshold = match user_threshold {
        Some(t) => t,
        None => {
            match address {
                Some(addr) => detect_script_type(addr).dust_threshold(),
                None => DUST_DEFAULT,
            }
        }
    };
    amount_sats < threshold
}

#[allow(dead_code)]
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

/// Classify using smart per-script-type thresholds
/// user_threshold=None means use per-type thresholds
/// user_threshold=Some(x) means override with custom value
pub fn classify_utxos_smart(
    utxos: Vec<ListUnspentResultEntry>,
    user_threshold: Option<u64>,
) -> (Vec<ListUnspentResultEntry>, Vec<ListUnspentResultEntry>) {
    let mut dust = vec![];
    let mut clean = vec![];

    for utxo in utxos {
        let address = utxo.address
    .as_ref()
    .map(|a| a.clone().assume_checked().to_string());

        let addr_str = address.as_deref();

        if is_dust_smart(utxo.amount.to_sat(), addr_str, user_threshold) {
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
    fn test_detect_script_type_p2pkh() {
        assert_eq!(detect_script_type("1A1zP1eP5QGefi2DMPTfTL5SLmv7Divf"), ScriptType::P2PKH);
    }

    #[test]
    fn test_detect_script_type_p2sh() {
        assert_eq!(detect_script_type("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy"), ScriptType::P2SH);
    }

    #[test]
    fn test_detect_script_type_p2wpkh() {
        assert_eq!(detect_script_type("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"), ScriptType::P2WPKH);
        assert_eq!(detect_script_type("bcrt1q60z267jynd9swr3dk7k3tmjqmlmsr8xsn4c8mz"), ScriptType::P2WPKH);
    }

    #[test]
    fn test_detect_script_type_p2tr() {
        assert_eq!(detect_script_type("bc1p0xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqzk5jj0"), ScriptType::P2TR);
    }

    #[test]
    fn test_dust_thresholds_per_type() {
        assert_eq!(ScriptType::P2PKH.dust_threshold(), 546);
        assert_eq!(ScriptType::P2WPKH.dust_threshold(), 294);
        assert_eq!(ScriptType::P2TR.dust_threshold(), 294);
        assert_eq!(ScriptType::P2SH.dust_threshold(), 540);
    }

    #[test]
    fn test_is_dust_smart_p2wpkh() {
        // 294 is NOT dust for P2WPKH (equal to threshold)
        assert!(!is_dust_smart(294, Some("bcrt1q60z267jynd9swr3dk7k3tmjqmlmsr8xsn4c8mz"), None));
        // 293 IS dust for P2WPKH
        assert!(is_dust_smart(293, Some("bcrt1q60z267jynd9swr3dk7k3tmjqmlmsr8xsn4c8mz"), None));
    }

    #[test]
    fn test_is_dust_smart_user_override() {
        // user sets threshold to 1000, overrides per-type
        assert!(is_dust_smart(500, Some("bcrt1q60z267jynd9swr3dk7k3tmjqmlmsr8xsn4c8mz"), Some(1000)));
        assert!(!is_dust_smart(1000, Some("bcrt1q60z267jynd9swr3dk7k3tmjqmlmsr8xsn4c8mz"), Some(1000)));
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