use bitcoincore_rpc::bitcoincore_rpc_json::ListUnspentResultEntry;
use bitcoincore_rpc::{Client, RpcApi};

pub struct SweepResult {
    pub psbt: String,
    pub dust_input_count: usize,
    pub total_dust_sats: u64,
}

pub struct DryRunResult {
    pub dust_input_count: usize,
    pub total_dust_sats: u64,
    pub funder_sats: u64,
    pub estimated_fee_sats: u64,
    pub estimated_output_sats: u64,
}

pub fn build_sweep_psbt(
    client: &Client,
    dust_utxos: &[ListUnspentResultEntry],
    clean_utxos: &[ListUnspentResultEntry],
) -> anyhow::Result<SweepResult> {
    if dust_utxos.is_empty() {
        anyhow::bail!("No dust UTXOs to sweep");
    }

    let funder = clean_utxos
        .iter()
        .max_by_key(|u| u.amount.to_sat())
        .ok_or_else(|| anyhow::anyhow!(
            "Cannot sweep: no clean UTXOs available to fund transaction fees.\nTip: fund your wallet first with a non-dust amount."
        ))?;

    let mut all_inputs: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "txid": funder.txid.to_string(),
            "vout": funder.vout,
        })
    ];

    for utxo in dust_utxos {
        all_inputs.push(serde_json::json!({
            "txid": utxo.txid.to_string(),
            "vout": utxo.vout,
        }));
    }

    let out_address = client.get_new_address(None, None)?;
    let out_address = out_address.assume_checked();

    let funder_btc = funder.amount.to_btc();
    let outputs = serde_json::json!([{
        out_address.to_string(): format!("{:.8}", funder_btc)
    }]);

    let response = client.call::<serde_json::Value>(
        "walletcreatefundedpsbt",
        &[
            serde_json::to_value(&all_inputs)?,
            outputs,
            serde_json::Value::Null,
            serde_json::json!({
                "subtractFeeFromOutputs": [0],
                "replaceable": true,
            }),
        ],
    )?;

    let psbt = response["psbt"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No PSBT returned from node"))?
        .to_string();

    let total_dust_sats = dust_utxos.iter().map(|u| u.amount.to_sat()).sum();

    Ok(SweepResult {
        psbt,
        dust_input_count: dust_utxos.len(),
        total_dust_sats,
    })
}

pub fn dry_run_sweep(
    dust_utxos: &[ListUnspentResultEntry],
    clean_utxos: &[ListUnspentResultEntry],
) -> anyhow::Result<DryRunResult> {
    if dust_utxos.is_empty() {
        anyhow::bail!("No dust UTXOs to sweep");
    }

    let funder = clean_utxos
        .iter()
        .max_by_key(|u| u.amount.to_sat())
        .ok_or_else(|| anyhow::anyhow!(
            "Cannot sweep: no clean UTXOs available to fund transaction fees."
        ))?;

    let total_dust_sats: u64 = dust_utxos.iter().map(|u| u.amount.to_sat()).sum();
    let funder_sats = funder.amount.to_sat();

    // 68 vbytes per P2WPKH input, 31 vbytes output, 10 vbytes overhead
    // fee rate: 2 sat/vbyte conservative estimate
    let total_inputs = dust_utxos.len() as u64 + 1;
    let estimated_vbytes = (total_inputs * 68) + 31 + 10;
    let estimated_fee_sats = estimated_vbytes * 2;
    let estimated_output_sats = funder_sats + total_dust_sats - estimated_fee_sats;

    Ok(DryRunResult {
        dust_input_count: dust_utxos.len(),
        total_dust_sats,
        funder_sats,
        estimated_fee_sats,
        estimated_output_sats,
    })
}